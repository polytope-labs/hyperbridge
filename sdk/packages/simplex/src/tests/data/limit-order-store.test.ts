import { describe, expect, it } from "vitest"
import { mkdtempSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
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
})

describe("SqliteLimitOrderStore", () => {
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
