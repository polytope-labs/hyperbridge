import { EventEmitter } from "node:events"
import { mkdtempSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { DatabaseSync } from "node:sqlite"
import { describe, expect, it } from "vitest"
import { INVENTORY_INTERVAL_MS, InventoryRecorder, inventoryOf } from "@/data/inventory"
import { MemoryDataStore } from "@/data/memory"
import { SqliteDataStore } from "@/data/sqlite"
import type { InventoryStore } from "@/data/types"
import type { BalanceSnapshot } from "@/services/BalanceProvider"
import { LoggerContext } from "@/services/Logger"

const dataDir = () => mkdtempSync(join(tmpdir(), "simplex-inventory-"))
const quiet = () => new LoggerContext({ level: "error" }).get("inventory")

/** One token on one chain, held partly in the wallet and partly in a vault. */
function asset(symbol: string, wallet: number, vault = 0) {
	return {
		address: `0x${symbol}`,
		symbol,
		wallet,
		walletReserve: 0,
		vaultPosition: vault,
		vaultAvailable: vault,
		total: wallet + vault,
		available: wallet + vault,
		vaults: [],
		status: "fresh" as const,
	}
}

function balances(overrides: Partial<BalanceSnapshot> = {}): BalanceSnapshot {
	return {
		updatedAt: Date.now(),
		status: "fresh",
		issues: [],
		chains: [
			{ chainId: 8453, assets: [asset("USDC", 400, 600), asset("cNGN", 1_000_000)] },
			{ chainId: 56, assets: [asset("USDC", 250)] },
		],
		...overrides,
	}
}

/** Both backends implement the same contract, so both run the same suite. */
const backends: [string, () => { store: InventoryStore; close: () => Promise<void> }][] = [
	[
		"SqliteInventoryStore",
		() => {
			const store = new SqliteDataStore(dataDir(), new LoggerContext({ level: "warn" }))
			return { store: store.inventory, close: () => store.close() }
		},
	],
	["MemoryInventoryStore", () => ({ store: new MemoryDataStore().inventory, close: async () => {} })],
]

describe.each(backends)("%s", (_name, open) => {
	it("has nothing until a snapshot is taken", async () => {
		const { store, close } = open()

		expect(await store.latest()).toBeNull()
		expect(await store.since(new Date(0))).toEqual([])
		await close()
	})

	it("keeps each snapshot whole, and hands back the latest", async () => {
		const { store, close } = open()
		await store.record({ USDC: 1250, cNGN: 1_000_000 })
		await store.record({ USDC: 1300.5 })

		const latest = await store.latest()
		expect(latest?.balances).toEqual({ USDC: 1300.5 })
		expect(latest?.takenAt).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/)

		const all = await store.since(new Date(0))
		expect(all.map((snapshot) => snapshot.balances)).toEqual([{ USDC: 1250, cNGN: 1_000_000 }, { USDC: 1300.5 }])
		await close()
	})

	it("lists only the snapshots taken since a given moment", async () => {
		const { store, close } = open()
		await store.record({ USDC: 1 })

		expect(await store.since(new Date(Date.now() - 60_000))).toHaveLength(1)
		expect(await store.since(new Date(Date.now() + 60_000))).toEqual([])
		await close()
	})
})

describe("SqliteInventoryStore", () => {
	it("adds its table to a data directory written before snapshots existed", async () => {
		const dir = dataDir()
		// An activity database as an earlier version left it: its own tables, and no inventory.
		const earlier = new DatabaseSync(join(dir, "activity.db"))
		earlier.exec(
			"CREATE TABLE events (id INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER NOT NULL, type TEXT NOT NULL)",
		)
		earlier.close()

		const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect(await store.inventory.latest()).toBeNull()
		await store.inventory.record({ USDC: 10 })
		await store.close()

		// And keeps what it recorded across a restart.
		const again = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		expect((await again.inventory.latest())?.balances).toEqual({ USDC: 10 })
		await again.close()
	})
})

describe("what a balance read says is held", () => {
	it("adds wallet and vault together, across every chain", () => {
		expect(inventoryOf(balances())).toEqual({ USDC: 1250, cNGN: 1_000_000 })
	})

	it("is nothing unless every balance was read", () => {
		// A chain that could not be read would look like a fall in inventory.
		expect(inventoryOf(balances({ status: "partial" }))).toBeNull()
		expect(inventoryOf(balances({ status: "loading", chains: [] }))).toBeNull()
		const unread = { ...asset("USDT", 5), total: null }
		expect(inventoryOf(balances({ chains: [{ chainId: 1, assets: [unread] }] }))).toBeNull()
		// And a solver holding no tokens has nothing to record.
		expect(inventoryOf(balances({ chains: [{ chainId: 1, assets: [] }] }))).toBeNull()
	})
})

/** What the recorder needs of the balance provider: its events, and its last read. */
type Balances = ConstructorParameters<typeof InventoryRecorder>[1]
const providerOf = (getSnapshot: () => BalanceSnapshot) =>
	Object.assign(new EventEmitter(), { getSnapshot }) as unknown as Balances & EventEmitter

describe("InventoryRecorder", () => {
	function setup(first: BalanceSnapshot = balances()) {
		const store = new MemoryDataStore().inventory
		let now = Date.parse("2026-10-05T12:00:00Z")
		let current = first
		const provider = providerOf(() => current)
		const recorder = new InventoryRecorder(store, provider, quiet(), () => now)
		return {
			store,
			recorder,
			advance: (ms: number) => {
				now += ms
			},
			read: async (next: BalanceSnapshot) => {
				current = next
				provider.emit("snapshot", next)
				// The recorder answers the event without being awaited; let it finish.
				await new Promise((resolve) => setTimeout(resolve, 0))
			},
			count: async () => (await store.since(new Date(0))).length,
		}
	}

	it("takes a snapshot from the first complete read", async () => {
		const ctx = setup()
		ctx.recorder.start()
		await new Promise((resolve) => setTimeout(resolve, 0))

		expect((await ctx.store.latest())?.balances).toEqual({ USDC: 1250, cNGN: 1_000_000 })
	})

	it("takes the next one a day later, however often balances are read", async () => {
		const ctx = setup()
		ctx.recorder.start()
		await ctx.read(balances())
		ctx.advance(INVENTORY_INTERVAL_MS - 1)
		await ctx.read(balances())
		expect(await ctx.count()).toBe(1)

		ctx.advance(1)
		await ctx.read(balances())
		expect(await ctx.count()).toBe(2)
	})

	it("waits for a complete read rather than recording a partial one", async () => {
		const ctx = setup(balances({ status: "loading", chains: [] }))
		ctx.recorder.start()
		await ctx.read(balances({ status: "partial" }))
		expect(await ctx.count()).toBe(0)

		await ctx.read(balances())
		expect(await ctx.count()).toBe(1)
	})

	it("does not take another on a restart inside the same day", async () => {
		const ctx = setup()
		await ctx.store.record({ USDC: 1 })
		// The store stamps the snapshot with the real clock; the recorder's clock starts there.
		const restarted = new InventoryRecorder(ctx.store, providerOf(balances), quiet())
		await restarted.consider(balances())

		expect(await ctx.count()).toBe(1)
	})

	it("stops listening when stopped", async () => {
		const ctx = setup(balances({ status: "loading", chains: [] }))
		ctx.recorder.start()
		ctx.recorder.stop()
		await ctx.read(balances())

		expect(await ctx.count()).toBe(0)
	})

	it("never lets a failed write reach the balance read that prompted it", async () => {
		const failing: InventoryStore = {
			record: async () => {
				throw new Error("disk full")
			},
			latest: async () => null,
			since: async () => [],
		}
		const recorder = new InventoryRecorder(failing, providerOf(balances), quiet())

		await expect(recorder.consider(balances())).resolves.toBeUndefined()
	})
})
