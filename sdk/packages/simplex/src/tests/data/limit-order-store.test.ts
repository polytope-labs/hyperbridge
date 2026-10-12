import { describe, expect, it } from "vitest"
import { mkdtempSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { DatabaseSync } from "node:sqlite"
import { ENTRY_POINT_V09 } from "@hyperbridge/sdk"
import { LoggerContext } from "@/services/Logger"
import { MemoryDataStore } from "@/data/memory"
import { SqliteDataStore } from "@/data/sqlite"
import type { LimitOrderInsert, LimitOrderStore } from "@/data/types"
import { summarizeProfit } from "@/orderbook/profitability"

const dataDir = () => mkdtempSync(join(tmpdir(), "simplex-limit-orders-"))

const ORDER: LimitOrderInsert = {
	id: "order-1",
	book: "USDC/CNGN",
	base: "USDC",
	quote: "CNGN",
	side: "BID",
	fillChain: "EVM-8453",
	price: "1500000000000000000000",
	size: "1500000000000000000000000",
	acceptedSources: ["EVM-1", "EVM-42161"],
	ttlSecs: 900,
}

/** Both backends implement the same contract, so both run the same suite. */
const backends: [string, () => { store: LimitOrderStore; close: () => Promise<void> }][] = [
	[
		"SqliteLimitOrderStore",
		() => {
			const store = new SqliteDataStore(dataDir(), new LoggerContext({ level: "warn" }))
			return { store: store.limitOrders, close: () => store.close() }
		},
	],
	["MemoryLimitOrderStore", () => ({ store: new MemoryDataStore().limitOrders, close: async () => {} })],
]

describe.each(backends)("%s", (_name, open) => {
	it("opens an order at its full size with nothing reserved", async () => {
		const { store, close } = open()
		const created = await store.create(ORDER)

		expect(created.status).toBe("open")
		expect(created.remaining).toBe(ORDER.size)
		expect(created.reserved).toBe("0")
		expect(created.commitment).toBeNull()
		expect(created.orderNonce).toBe("0")
		expect(created.entryPoint).toBeNull()
		expect(created.acceptedSources).toEqual(["EVM-1", "EVM-42161"])
		await close()
	})

	it("keeps 1e18 amounts exact, past what a 64-bit integer column would hold", async () => {
		const { store, close } = open()
		// Well past 2^63, which is where a numeric column would start rounding.
		const size = "123456789012345678901234567890"
		const created = await store.create({ ...ORDER, size, price: size })

		expect(created.size).toBe(size)
		expect((await store.get(ORDER.id))?.price).toBe(size)
		await close()
	})

	it("records a posting and reads it back", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		const posted = await store.setPosting(ORDER.id, {
			commitment: "0xabc",
			bookExpiresAt: "2026-09-15T12:00:00.000Z",
			bookPrice: "1490000000000000000000",
			orderNonce: "3",
			entryPoint: ENTRY_POINT_V09,
			status: "open",
			lastError: null,
		})

		expect(posted?.commitment).toBe("0xabc")
		expect(posted?.orderNonce).toBe("3")
		expect(posted?.bookPrice).toBe("1490000000000000000000")
		expect(posted?.entryPoint).toBe(ENTRY_POINT_V09)
		await close()
	})

	it("keeps a posting's EntryPoint until a new posting replaces it", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		const posting = {
			commitment: "0xabc",
			bookExpiresAt: null,
			bookPrice: null,
			orderNonce: "1",
			entryPoint: ENTRY_POINT_V09,
			status: "open" as const,
			lastError: null,
		}
		await store.setPosting(ORDER.id, posting)

		// A repost marks the row before the new posting lands, and the old one is still up meanwhile.
		expect((await store.setStatus(ORDER.id, "resizing"))?.entryPoint).toBe(ENTRY_POINT_V09)

		const cleared = await store.setPosting(ORDER.id, { ...posting, commitment: null, entryPoint: null })
		expect(cleared?.entryPoint).toBeNull()
		await close()
	})

	it("filters a listing by status, chain and book", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		await store.create({ ...ORDER, id: "order-2", fillChain: "EVM-1" })
		await store.setStatus("order-2", "cancelled")

		expect((await store.list({ status: "open" })).map((order) => order.id)).toEqual(["order-1"])
		expect((await store.list({ fillChain: "EVM-1" })).map((order) => order.id)).toEqual(["order-2"])
		expect(await store.list({ book: "USDC/EURC" })).toEqual([])
		expect(await store.list()).toHaveLength(2)
		await close()
	})

	it("carries a rejection on the row instead of losing it", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		const rejected = await store.setStatus(ORDER.id, "rejected", "UNSUPPORTED_PAIR: no such market")

		expect(rejected?.status).toBe("rejected")
		expect(rejected?.lastError).toBe("UNSUPPORTED_PAIR: no such market")
		await close()
	})

	it("resolves null for an id it does not know", async () => {
		const { store, close } = open()
		expect(await store.get("missing")).toBeNull()
		expect(await store.setStatus("missing", "cancelled")).toBeNull()
		await close()
	})

	describe("reserve", () => {
		it("holds output against the order and reports what is left", async () => {
			const { store, close } = open()
			await store.create(ORDER)

			expect(await store.reserve(ORDER.id, "1000")).toBe(true)
			expect(await store.reserve(ORDER.id, "500")).toBe(true)
			expect((await store.get(ORDER.id))?.reserved).toBe("1500")
			await close()
		})

		it("does not count other bids' holds against what the order has left", async () => {
			// A pending bid must not stop the next one going out.
			const { store, close } = open()
			await store.create({ ...ORDER, size: "1000" })

			expect(await store.reserve(ORDER.id, "600")).toBe(true)
			expect(await store.reserve(ORDER.id, "500")).toBe(true)
			expect((await store.get(ORDER.id))?.reserved).toBe("1100")
			await close()
		})

		it("refuses a single hold larger than what the order has left", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "1000" })

			expect(await store.reserve(ORDER.id, "1001")).toBe(false)
			expect((await store.get(ORDER.id))?.reserved).toBe("0")
			await close()
		})

		it("records every hold when several callers reserve against one order", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "1000" })

			// Five chains bidding at once, each wanting most of the order. Every hold
			// goes through and none is lost; the cross-connection test below is what
			// exercises the database guard itself.
			const results = await Promise.all([600, 600, 600, 600, 600].map(() => store.reserve(ORDER.id, "600")))

			expect(results.every(Boolean)).toBe(true)
			expect((await store.get(ORDER.id))?.reserved).toBe("3000")
			await close()
		})

		it("refuses an order that is no longer open", async () => {
			const { store, close } = open()
			await store.create(ORDER)
			await store.setStatus(ORDER.id, "cancelled")

			expect(await store.reserve(ORDER.id, "1000")).toBe(false)
			expect(await store.reserve("missing", "1000")).toBe(false)
			await close()
		})
	})

	describe("transaction", () => {
		it("keeps a settlement that throws part-way from landing at all", async () => {
			// A fill claims what its bid held, works the orders down and gives the rest
			// back. Half of that is worse than none: a hold released against an order
			// that was never drawn down leaves it advertising output already paid.
			const { store, close } = open()
			try {
				await store.create(ORDER)
				await store.reserve(ORDER.id, "100")

				await expect(
					store.transaction(async () => {
						await store.drawDown(ORDER.id, "100")
						await store.release(ORDER.id, "100")
						throw new Error("the fill path exploded")
					}),
				).rejects.toThrow("exploded")

				const after = await store.get(ORDER.id)
				expect(after?.remaining).toBe(ORDER.size)
				expect(after?.reserved).toBe("100")
			} finally {
				await close()
			}
		})

		it("commits every write when the settlement finishes", async () => {
			const { store, close } = open()
			try {
				await store.create(ORDER)
				await store.reserve(ORDER.id, "100")

				await store.transaction(async () => {
					await store.drawDown(ORDER.id, "100")
					await store.release(ORDER.id, "100")
				})

				const after = await store.get(ORDER.id)
				expect(after?.remaining).toBe((BigInt(ORDER.size) - 100n).toString())
				expect(after?.reserved).toBe("0")
			} finally {
				await close()
			}
		})
	})

	describe("release", () => {
		it("gives a reservation back", async () => {
			const { store, close } = open()
			await store.create(ORDER)
			await store.reserve(ORDER.id, "1000")
			await store.release(ORDER.id, "400")

			expect((await store.get(ORDER.id))?.reserved).toBe("600")
			await close()
		})

		it("floors at zero, so a double release cannot invent capacity", async () => {
			const { store, close } = open()
			await store.create(ORDER)
			await store.reserve(ORDER.id, "1000")
			await store.release(ORDER.id, "1000")
			await store.release(ORDER.id, "1000")

			expect((await store.get(ORDER.id))?.reserved).toBe("0")
			await close()
		})
	})

	describe("clampRemaining", () => {
		it("lowers remaining to the room the chain has left", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })

			const clamped = await store.clampRemaining(ORDER.id, "60")

			expect(clamped?.id).toBe(ORDER.id)
			expect(clamped?.remaining).toBe("60")
			expect((await store.get(ORDER.id))?.remaining).toBe("60")
			await close()
		})

		it("never raises remaining", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			await store.drawDown(ORDER.id, "60")

			expect(await store.clampRemaining(ORDER.id, "60")).toBeNull()
			expect((await store.get(ORDER.id))?.remaining).toBe("40")
			await close()
		})

		it("counts what is reserved as room still spoken for", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			await store.reserve(ORDER.id, "30")

			expect(await store.clampRemaining(ORDER.id, "70")).toBeNull()
			expect((await store.get(ORDER.id))?.remaining).toBe("100")

			expect((await store.clampRemaining(ORDER.id, "60"))?.remaining).toBe("90")
			expect((await store.get(ORDER.id))?.reserved).toBe("30")
			await close()
		})

		it("does not take a fill off twice when the chain counted it before it settled here", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			await store.reserve(ORDER.id, "25")

			expect(await store.clampRemaining(ORDER.id, "75")).toBeNull()
			expect((await store.get(ORDER.id))?.remaining).toBe("100")

			await store.transaction(async () => {
				await store.drawDown(ORDER.id, "25")
				await store.release(ORDER.id, "25")
			})
			const settled = await store.get(ORDER.id)
			expect(settled?.remaining).toBe("75")
			expect(settled?.reserved).toBe("0")

			expect(await store.clampRemaining(ORDER.id, "75")).toBeNull()
			expect((await store.get(ORDER.id))?.remaining).toBe("75")
			await close()
		})

		it.each(["resizing", "filled", "cancelled", "expired", "rejected"] as const)(
			"leaves a %s order alone",
			async (status) => {
				const { store, close } = open()
				await store.create({ ...ORDER, size: "100" })
				await store.reserve(ORDER.id, "10")
				const before = await store.setStatus(ORDER.id, status)

				expect(await store.clampRemaining(ORDER.id, "0")).toBeNull()
				expect(await store.get(ORDER.id)).toEqual(before)
				await close()
			},
		)

		it("resolves null for an id it does not know", async () => {
			const { store, close } = open()
			expect(await store.clampRemaining("missing", "0")).toBeNull()
			await close()
		})

		it("clamps to zero when the chain has no room and nothing is reserved, without closing the order", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })

			const clamped = await store.clampRemaining(ORDER.id, "0")

			expect(clamped?.remaining).toBe("0")
			expect(clamped?.status).toBe("open")
			await close()
		})

		it("changes nothing but remaining, and records no fill", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			await store.reserve(ORDER.id, "30")
			const before = await store.get(ORDER.id)

			const clamped = await store.clampRemaining(ORDER.id, "20")

			expect(clamped).toEqual({ ...before, remaining: "50", updatedAt: clamped?.updatedAt })
			expect(await store.fills(ORDER.id)).toEqual([])
			await close()
		})
	})
})

describe("SqliteLimitOrderStore", () => {
	it("adds another connection's hold to the same order's total", async () => {
		// Two solvers sharing a data directory. The total has to live in the
		// database, not in one process's memory, or each would overwrite the other.
		const dir = dataDir()
		const first = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		const second = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))

		await first.limitOrders.create({ ...ORDER, size: "1000" })
		expect(await first.limitOrders.reserve(ORDER.id, "600")).toBe(true)
		expect(await second.limitOrders.reserve(ORDER.id, "600")).toBe(true)
		expect(await second.limitOrders.reserve(ORDER.id, "1001")).toBe(false)
		expect((await first.limitOrders.get(ORDER.id))?.reserved).toBe("1200")

		await first.close()
		await second.close()
	})

	it("refuses an update whose reservation moved under it", async () => {
		// `reserve` reads, decides, then writes guarded on the value it read. This
		// drives that gap by hand: the write is stale, so it must not land.
		const dir = dataDir()
		const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		await store.limitOrders.create({ ...ORDER, size: "1000" })
		await store.limitOrders.reserve(ORDER.id, "500")

		const db = new DatabaseSync(join(dir, "bids.db"))
		const stale = db
			.prepare("UPDATE limit_orders SET reserved = ? WHERE id = ? AND reserved = ? AND status = 'open'")
			.run("900", ORDER.id, "0")
		db.close()

		expect(stale.changes).toBe(0)
		expect((await store.limitOrders.get(ORDER.id))?.reserved).toBe("500")
		await store.close()
	})

	it("adds entry_point in place to a database written before it, reading its postings as null", async () => {
		const dir = dataDir()
		const legacy = new DatabaseSync(join(dir, "bids.db"))
		legacy.exec(`
			CREATE TABLE limit_orders (
				id TEXT PRIMARY KEY,
				book TEXT NOT NULL,
				base TEXT NOT NULL,
				quote TEXT NOT NULL,
				side TEXT NOT NULL,
				fill_chain TEXT NOT NULL,
				price TEXT NOT NULL,
				size TEXT NOT NULL,
				remaining TEXT NOT NULL,
				reserved TEXT NOT NULL DEFAULT '0',
				accepted_sources TEXT NOT NULL,
				ttl_secs INTEGER NOT NULL,
				expires_at TEXT,
				status TEXT NOT NULL,
				commitment TEXT,
				order_nonce TEXT NOT NULL DEFAULT '0',
				book_expires_at TEXT,
				book_price TEXT,
				last_error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				updated_at TEXT NOT NULL DEFAULT (datetime('now'))
			);
		`)
		legacy
			.prepare(
				`
				INSERT INTO limit_orders (
					id, book, base, quote, side, fill_chain, price, size, remaining,
					accepted_sources, ttl_secs, status, commitment, order_nonce
				)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'open', '0xa1', '4')
			`,
			)
			.run(
				ORDER.id,
				ORDER.book,
				ORDER.base,
				ORDER.quote,
				ORDER.side,
				ORDER.fillChain,
				ORDER.price,
				ORDER.size,
				ORDER.size,
				JSON.stringify(ORDER.acceptedSources),
				ORDER.ttlSecs,
			)
		legacy.close()

		const migrated = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect(await migrated.limitOrders.get(ORDER.id)).toMatchObject({
			status: "open",
			commitment: "0xa1",
			orderNonce: "4",
			remaining: ORDER.size,
			acceptedSources: ORDER.acceptedSources,
			entryPoint: null,
		})
		await migrated.limitOrders.setPosting(ORDER.id, {
			commitment: "0xa2",
			bookExpiresAt: null,
			bookPrice: null,
			orderNonce: "5",
			entryPoint: ENTRY_POINT_V09,
			status: "open",
			lastError: null,
		})
		await migrated.close()

		const reopened = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect(await reopened.limitOrders.get(ORDER.id)).toMatchObject({
			commitment: "0xa2",
			entryPoint: ENTRY_POINT_V09,
		})
		await reopened.close()
	})

	it("survives a reopen of the same data directory", async () => {
		const dir = dataDir()
		const first = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		await first.limitOrders.create(ORDER)
		await first.close()

		const second = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect((await second.limitOrders.get(ORDER.id))?.size).toBe(ORDER.size)
		await second.close()
	})
})

describe("SqliteLimitOrderStore fill migration", () => {
	const columnsOf = (dir: string) => {
		const db = new DatabaseSync(join(dir, "bids.db"))
		const columns = db.prepare("PRAGMA table_info(limit_order_fills)").all() as unknown as Array<{ name: string }>
		db.close()
		return columns.map((column) => column.name)
	}

	/**
	 * A data directory as the release before left it: both limit order tables at the schema that
	 * shipped, a buy and a sell, and a fill of each. Written with plain SQL rather than through
	 * the store, which could only ever write the schema it has now.
	 */
	function shippedDataDir(): string {
		const dir = dataDir()
		const db = new DatabaseSync(join(dir, "bids.db"))
		db.exec(`
			CREATE TABLE limit_orders (
				id TEXT PRIMARY KEY,
				book TEXT NOT NULL,
				base TEXT NOT NULL,
				quote TEXT NOT NULL,
				side TEXT NOT NULL,
				fill_chain TEXT NOT NULL,
				price TEXT NOT NULL,
				size TEXT NOT NULL,
				remaining TEXT NOT NULL,
				reserved TEXT NOT NULL DEFAULT '0',
				accepted_sources TEXT NOT NULL,
				ttl_secs INTEGER NOT NULL,
				expires_at TEXT,
				status TEXT NOT NULL,
				commitment TEXT,
				order_nonce TEXT NOT NULL DEFAULT '0',
				book_expires_at TEXT,
				book_price TEXT,
				last_error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				updated_at TEXT NOT NULL DEFAULT (datetime('now'))
			);
			CREATE TABLE limit_order_fills (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				limit_order_id TEXT NOT NULL,
				commitment TEXT NOT NULL,
				bid TEXT,
				amount TEXT NOT NULL,
				transaction_hash TEXT,
				filled_at TEXT NOT NULL DEFAULT (datetime('now'))
			);
			CREATE INDEX idx_limit_order_fills_order ON limit_order_fills(limit_order_id);

			INSERT INTO limit_orders (id, book, base, quote, side, fill_chain, price, size, remaining, accepted_sources, ttl_secs, status)
			VALUES
				('buy', 'USDC-cNGN', 'USDC', 'cNGN', 'BID', 'EVM-8453', '1500000000000000000000', '3000000000000000000000', '1500000000000000000000', '["EVM-1"]', 900, 'open'),
				('sell', 'USDC-cNGN', 'USDC', 'cNGN', 'ASK', 'EVM-8453', '1510000000000000000000', '2000000000000000000', '1000000000000000000', '["EVM-1"]', 900, 'open');
			-- The buy paid 1,500 cNGN out and the sell paid 1 USDC out. Neither says what it took in.
			INSERT INTO limit_order_fills (limit_order_id, commitment, bid, amount, transaction_hash, filled_at)
			VALUES
				('buy', '0xb1', '0xbid1', '1500000000000000000000', '0xt1', '2026-10-01 10:00:00'),
				('sell', '0xs1', NULL, '1000000000000000000', NULL, '2026-10-02 10:00:00');
		`)
		db.close()
		return dir
	}

	it("adds what a fill took in to a database the release before wrote, and keeps its rows", async () => {
		const dir = shippedDataDir()
		expect(columnsOf(dir)).not.toContain("amount_in")

		const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect(columnsOf(dir)).toContain("amount_in")

		// Every fill it held is still there, with nothing in the new column.
		expect(
			(await store.limitOrders.fills("buy")).map((fill) => [
				fill.commitment,
				fill.bid,
				fill.amount,
				fill.amountIn,
			]),
		).toEqual([["0xb1", "0xbid1", "1500000000000000000000", null]])
		expect((await store.limitOrders.get("sell"))?.remaining).toBe("1000000000000000000")

		// And a fill settled from here on keeps what it took in.
		await store.limitOrders.recordFill({
			limitOrderId: "sell",
			commitment: "0xs2",
			amount: "1000000000000000000",
			amountIn: "1512000000000000000000",
		})
		expect((await store.limitOrders.fillHistory()).map((fill) => [fill.limitOrderId, fill.amountIn])).toEqual([
			["buy", null],
			["sell", null],
			["sell", "1512000000000000000000"],
		])
		await store.close()
	})

	it("prices the fills it migrated at their order's rate, and says they are estimates", async () => {
		const store = new SqliteDataStore(shippedDataDir(), new LoggerContext({ level: "warn" }))
		const summary = summarizeProfit(await store.limitOrders.fillHistory(), {
			period: "7d",
			now: Date.parse("2026-10-05T12:00:00Z"),
		})

		// 1 USDC bought at 1,500 and sold at 1,510, each at its order's own rate: 10 cNGN up.
		expect(summary.books[0]).toMatchObject({
			bought: 1,
			sold: 1,
			averageBuy: 1500,
			averageSell: 1510,
			baseChange: 0,
			quoteChange: 10,
		})
		expect(summary.estimatedFills).toBe(2)
		await store.close()
	})

	it("migrates once: opening the database again changes nothing", async () => {
		const dir = shippedDataDir()
		await new SqliteDataStore(dir, new LoggerContext({ level: "warn" })).close()
		const migrated = columnsOf(dir)

		const again = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect(columnsOf(dir)).toEqual(migrated)
		expect(await again.limitOrders.fillHistory()).toHaveLength(2)
		await again.close()
	})

	it("leaves the database writable by a solver still on the release before", async () => {
		// Two solvers can share a data directory, and they are not upgraded in the same instant.
		const dir = shippedDataDir()
		const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))

		// The earlier version's insert, which names no `amount_in`.
		const older = new DatabaseSync(join(dir, "bids.db"))
		older
			.prepare(
				"INSERT INTO limit_order_fills (limit_order_id, commitment, bid, amount, transaction_hash) VALUES (?, ?, ?, ?, ?)",
			)
			.run("buy", "0xb2", null, "750000000000000000000", null)
		older.close()

		expect((await store.limitOrders.fills("buy")).map((fill) => [fill.commitment, fill.amountIn])).toEqual([
			["0xb2", null],
			["0xb1", null],
		])
		await store.close()
	})

	it("creates a new database with the column already in place, not by migrating to it", async () => {
		const dir = dataDir()
		await new SqliteDataStore(dir, new LoggerContext({ level: "warn" })).close()

		// A migration appends its column; a table created at this schema has it beside `amount`.
		const columns = columnsOf(dir)
		expect(columns.indexOf("amount_in")).toBe(columns.indexOf("amount") + 1)
	})
})

describe.each(backends)("%s fill history", (_name, open) => {
	it("keeps every fill against its order, newest first, across a repost", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xaa", bid: "0xb1", amount: "100", transactionHash: "0xt1" })
		// A resize reposts the order under a new commitment; its history must not move.
		await store.setPosting(ORDER.id, {
			commitment: "0xnew",
			bookExpiresAt: null,
			bookPrice: null,
			orderNonce: "1",
			entryPoint: null,
			status: "open",
			lastError: null,
		})
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xbb", amount: "50" })
		await store.recordFill({ limitOrderId: "someone-else", commitment: "0xcc", amount: "7" })

		const fills = await store.fills(ORDER.id)
		expect(fills.map((fill) => [fill.commitment, fill.amount, fill.bid, fill.transactionHash])).toEqual([
			["0xbb", "50", null, null],
			["0xaa", "100", "0xb1", "0xt1"],
		])
		expect(fills.every((fill) => fill.limitOrderId === ORDER.id && typeof fill.filledAt === "string")).toBe(true)
		await close()
	})

	it("keeps what a fill took in beside what it paid out", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xaa", amount: "1500", amountIn: "1" })
		// A fill whose event carried no inputs has nothing to keep.
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xbb", amount: "3000" })

		expect((await store.fills(ORDER.id)).map((fill) => [fill.commitment, fill.amountIn])).toEqual([
			["0xbb", null],
			["0xaa", "1"],
		])
		await close()
	})

	it("reads every order's fills back with its terms, oldest first", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		await store.create({ ...ORDER, id: "order-2", side: "ASK", price: "1510000000000000000000" })
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xaa", amount: "1500", amountIn: "1" })
		await store.recordFill({ limitOrderId: "order-2", commitment: "0xbb", amount: "2" })
		// A fill whose order is not on record could not be priced, and is left out.
		await store.recordFill({ limitOrderId: "someone-else", commitment: "0xcc", amount: "7" })

		const history = await store.fillHistory()
		expect(history.map(({ id, filledAt, ...terms }) => terms)).toEqual([
			{
				limitOrderId: ORDER.id,
				book: ORDER.book,
				base: "USDC",
				quote: "CNGN",
				side: "BID",
				price: ORDER.price,
				amount: "1500",
				amountIn: "1",
			},
			{
				limitOrderId: "order-2",
				book: ORDER.book,
				base: "USDC",
				quote: "CNGN",
				side: "ASK",
				price: "1510000000000000000000",
				amount: "2",
				amountIn: null,
			},
		])
		expect(history.every((fill) => typeof fill.filledAt === "string" && fill.id > 0)).toBe(true)
		await close()
	})

	it("drops a fill recorded inside a settlement that failed", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		await expect(
			store.transaction(async () => {
				await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xaa", amount: "100" })
				throw new Error("settlement failed")
			}),
		).rejects.toThrow("settlement failed")

		expect(await store.fills(ORDER.id)).toEqual([])
		await close()
	})
})

