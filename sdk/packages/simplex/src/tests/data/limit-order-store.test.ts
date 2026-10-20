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
	it("opens an order at its full size", async () => {
		const { store, close } = open()
		const created = await store.create(ORDER)

		expect(created.status).toBe("open")
		expect(created.remaining).toBe(ORDER.size)
		expect(created).not.toHaveProperty("reserved")
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
			await store.clampRemaining(ORDER.id, "40")

			expect(await store.clampRemaining(ORDER.id, "60")).toBeNull()
			expect((await store.get(ORDER.id))?.remaining).toBe("40")
			await close()
		})

		it("lowers a resizing order, which is how a repost applies a fill that landed meanwhile", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			await store.setStatus(ORDER.id, "resizing")

			const clamped = await store.clampRemaining(ORDER.id, "60")

			expect(clamped?.remaining).toBe("60")
			expect(clamped?.status).toBe("resizing")
			expect((await store.get(ORDER.id))?.remaining).toBe("60")
			await close()
		})

		it("leaves a resizing order alone when only an open one may be clamped", async () => {
			// A fill's sync passes ["open"]: a repost already holds the row, and a second
			// repost of it would orphan a book entry.
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			const held = await store.setStatus(ORDER.id, "resizing")

			expect(await store.clampRemaining(ORDER.id, "60", ["open"])).toBeNull()
			expect(await store.get(ORDER.id)).toEqual(held)
			await close()
		})

		it("lowers a resizing order, and leaves an open one, when only a resizing one may be clamped", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })

			expect(await store.clampRemaining(ORDER.id, "60", ["resizing"])).toBeNull()
			expect((await store.get(ORDER.id))?.remaining).toBe("100")

			await store.setStatus(ORDER.id, "resizing")
			expect((await store.clampRemaining(ORDER.id, "60", ["resizing"]))?.remaining).toBe("60")
			await close()
		})

		it("lowers an open order when only an open one may be clamped", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })

			expect((await store.clampRemaining(ORDER.id, "60", ["open"]))?.remaining).toBe("60")
			await close()
		})

		it("never widens the statuses: a closed order stays alone whatever `only` lists", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "100" })
			const before = await store.setStatus(ORDER.id, "filled")

			expect(await store.clampRemaining(ORDER.id, "0", ["filled"])).toBeNull()
			expect(await store.get(ORDER.id)).toEqual(before)
			await close()
		})

		it.each(["filled", "cancelled", "expired", "rejected"] as const)(
			"leaves a %s order alone",
			async (status) => {
				const { store, close } = open()
				await store.create({ ...ORDER, size: "100" })
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

		it("clamps to zero when the chain has no room, without closing the order", async () => {
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
			const before = await store.get(ORDER.id)

			const clamped = await store.clampRemaining(ORDER.id, "20")

			expect(clamped).toEqual({ ...before, remaining: "20", updatedAt: clamped?.updatedAt })
			expect(await store.fills(ORDER.id)).toEqual([])
			await close()
		})
	})
})

describe("SqliteLimitOrderStore", () => {
	it("neither reads nor writes the reserved column an older build left, but keeps it for a rollback", async () => {
		const dir = dataDir()
		const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		await store.limitOrders.create({ ...ORDER, size: "1000" })
		// A hold the build before this one took and never gave back.
		const db = new DatabaseSync(join(dir, "bids.db"))
		db.prepare("UPDATE limit_orders SET reserved = '250' WHERE id = ?").run(ORDER.id)

		// Clamped to the chain's room alone; the stale hold is not added back.
		expect((await store.limitOrders.clampRemaining(ORDER.id, "600"))?.remaining).toBe("600")
		expect(await store.limitOrders.get(ORDER.id)).not.toHaveProperty("reserved")

		const row = db.prepare("SELECT remaining, reserved FROM limit_orders WHERE id = ?").get(ORDER.id)
		db.close()
		await store.close()
		expect(row).toEqual({ remaining: "600", reserved: "250" })
	})

	it("clears a stale reserved on every start, so a rollback does not inherit a hold", async () => {
		const dir = dataDir()
		const first = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		await first.limitOrders.create({ ...ORDER, size: "1000" })
		await first.limitOrders.create({ ...ORDER, id: "order-2", size: "1000" })
		await first.close()

		const reservedOf = () => {
			const db = new DatabaseSync(join(dir, "bids.db"))
			const rows = db.prepare("SELECT id, reserved FROM limit_orders ORDER BY id").all()
			db.close()
			return rows
		}
		// Holds a build that kept them took and, once this build retracted their bids, never gave back.
		const db = new DatabaseSync(join(dir, "bids.db"))
		db.prepare("UPDATE limit_orders SET reserved = '250' WHERE id = ?").run(ORDER.id)
		db.close()
		expect(reservedOf()).toEqual([
			{ id: ORDER.id, reserved: "250" },
			{ id: "order-2", reserved: "0" },
		])

		const reopened = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect(reservedOf()).toEqual([
			{ id: ORDER.id, reserved: "0" },
			{ id: "order-2", reserved: "0" },
		])
		// Nothing else about the order moved.
		expect((await reopened.limitOrders.get(ORDER.id))?.remaining).toBe("1000")
		await reopened.close()

		// And again: nothing left to clear, nothing written.
		const again = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		await again.close()
		expect(reservedOf().every((row) => row.reserved === "0")).toBe(true)
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
})

describe.each(backends)("%s fill log index", (_name, open) => {
	it("records which log of its transaction a fill came from, and reads null when none was given", async () => {
		const { store, close } = open()
		await store.create(ORDER)
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xaa", amount: "1", transactionHash: "0xt", logIndex: 4 })
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xaa", amount: "2", transactionHash: "0xt", logIndex: 7 })
		await store.recordFill({ limitOrderId: ORDER.id, commitment: "0xbb", amount: "3" })

		expect((await store.fills(ORDER.id)).map((fill) => [fill.amount, fill.transactionHash, fill.logIndex])).toEqual([
			["3", null, null],
			["2", "0xt", 7],
			["1", "0xt", 4],
		])
		await close()
	})
})

describe("SqliteLimitOrderStore log_index migration", () => {
	it("adds log_index in place, keeps older fills as null, and stays writable by the build before", async () => {
		const dir = dataDir()
		const db = new DatabaseSync(join(dir, "bids.db"))
		db.exec(`
			CREATE TABLE limit_order_fills (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				limit_order_id TEXT NOT NULL,
				commitment TEXT NOT NULL,
				bid TEXT,
				amount TEXT NOT NULL,
				amount_in TEXT,
				transaction_hash TEXT,
				filled_at TEXT NOT NULL DEFAULT (datetime('now'))
			);
			INSERT INTO limit_order_fills (limit_order_id, commitment, amount, transaction_hash) VALUES ('o', '0xaa', '5', '0xt');
		`)
		db.close()

		const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect((await store.limitOrders.fills("o")).map((fill) => [fill.amount, fill.logIndex])).toEqual([["5", null]])
		await store.limitOrders.recordFill({ limitOrderId: "o", commitment: "0xbb", amount: "6", logIndex: 2 })
		await store.close()

		const older = new DatabaseSync(join(dir, "bids.db"))
		older
			.prepare("INSERT INTO limit_order_fills (limit_order_id, commitment, amount) VALUES (?, ?, ?)")
			.run("o", "0xcc", "7")
		const columns = older.prepare("PRAGMA table_info(limit_order_fills)").all() as unknown as Array<{ name: string }>
		older.close()
		expect(columns.map((column) => column.name)).toContain("log_index")
	})
})
