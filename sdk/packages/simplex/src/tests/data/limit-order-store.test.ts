import { describe, expect, it } from "vitest"
import { mkdtempSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { DatabaseSync } from "node:sqlite"
import { LoggerContext } from "@/services/Logger"
import { MemoryDataStore } from "@/data/memory"
import { SqliteDataStore } from "@/data/sqlite"
import type { LimitOrderInsert, LimitOrderStore } from "@/data/types"

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
			status: "open",
			lastError: null,
		})

		expect(posted?.commitment).toBe("0xabc")
		expect(posted?.orderNonce).toBe("3")
		expect(posted?.bookPrice).toBe("1490000000000000000000")
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

		it("refuses to reserve more than the order has left", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "1000" })

			expect(await store.reserve(ORDER.id, "600")).toBe(true)
			expect(await store.reserve(ORDER.id, "500")).toBe(false)
			expect((await store.get(ORDER.id))?.reserved).toBe("600")
			await close()
		})

		it("holds the cap when several callers reserve against one order", async () => {
			const { store, close } = open()
			await store.create({ ...ORDER, size: "1000" })

			// Five chains bidding, each wanting most of the order. Both backends run
			// synchronously underneath, so this pins the invariant rather than an
			// interleaving; the cross-connection test below is what exercises the
			// database guard itself.
			const results = await Promise.all([600, 600, 600, 600, 600].map(() => store.reserve(ORDER.id, "600")))

			expect(results.filter(Boolean)).toHaveLength(1)
			expect((await store.get(ORDER.id))?.reserved).toBe("600")
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
})

describe("SqliteLimitOrderStore", () => {
	it("counts another connection's reservation against the same order", async () => {
		// Two solvers sharing a data directory. The guard has to live in the
		// database, not in one process's memory, or each would see the full size.
		const dir = dataDir()
		const first = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		const second = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))

		await first.limitOrders.create({ ...ORDER, size: "1000" })
		expect(await first.limitOrders.reserve(ORDER.id, "600")).toBe(true)
		expect(await second.limitOrders.reserve(ORDER.id, "600")).toBe(false)
		expect(await second.limitOrders.reserve(ORDER.id, "400")).toBe(true)
		expect((await first.limitOrders.get(ORDER.id))?.reserved).toBe("1000")

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
