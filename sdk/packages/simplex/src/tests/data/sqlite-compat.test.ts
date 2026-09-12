import { afterEach, describe, expect, it } from "vitest"
import { cpSync, mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { DatabaseSync } from "node:sqlite"
import { fileURLToPath } from "node:url"
import { SqliteDataStore } from "@/data/sqlite"
import type { OrderSummary, SimplexDataStore } from "@/data/types"

/**
 * Backwards compatibility for operator data directories (#1236).
 *
 * The store moved from better-sqlite3 to `node:sqlite`. Operators have `bids.db`
 * and `activity.db` files already on disk, and a bid row is how a locked deposit
 * is found again for retraction — losing one strands funds. So the fixtures under
 * `fixtures/legacy-v0` are real database files *written by better-sqlite3*, at
 * the schema that shipped before the in-place column migrations
 * (`dead`/`pending`, `order_json`, `token_in`/`amount_in`). Regenerate them with
 * `scripts/make-legacy-db-fixture.mjs`.
 *
 * A database this store created itself could not prove any of this: it would
 * already have every column, and it would have been written by the same driver
 * that reads it back.
 */

const FIXTURE = fileURLToPath(new URL("./fixtures/legacy-v0", import.meta.url))

/** The rows in the fixture, so the expectations below read as data, not magic. */
const WON = `0x${"aa".repeat(32)}`
const RETRACTED = `0x${"bb".repeat(32)}`
const FAILED = `0x${"cc".repeat(32)}`

const dirs: string[] = []
const stores: SimplexDataStore[] = []

/** A throwaway copy of the fixture directory — the committed files stay pristine. */
function legacyDataDir(): string {
	const dir = mkdtempSync(join(tmpdir(), "simplex-legacy-"))
	dirs.push(dir)
	cpSync(FIXTURE, dir, { recursive: true })
	return dir
}

function openLegacyStore(): { store: SimplexDataStore; dir: string } {
	const dir = legacyDataDir()
	const store = new SqliteDataStore(dir)
	stores.push(store)
	return { store, dir }
}

afterEach(async () => {
	for (const store of stores.splice(0)) await store.close?.()
	for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true })
})

describe("SqliteDataStore: data directories written by better-sqlite3", () => {
	it("reads bids that predate the dead and pending columns", async () => {
		const { store } = openLegacyStore()

		const won = await store.bids.byCommitment(WON)
		expect(won).not.toBeNull()
		expect(won).toMatchObject({
			commitment: WON,
			extrinsicHash: `0x${"11".repeat(32)}`,
			blockHash: `0x${"22".repeat(32)}`,
			success: true,
			createdAt: "2025-01-02 03:04:05",
			retracted: false,
			error: null,
		})
		// Columns the migration added, defaulted for rows written before they existed.
		expect(won!.dead).toBe(false)
		expect(won!.pending).toBe(false)

		const retracted = await store.bids.byCommitment(RETRACTED)
		expect(retracted).toMatchObject({
			retracted: true,
			retractedAt: "2025-01-02 04:00:00",
			retractExtrinsicHash: `0x${"55".repeat(32)}`,
		})

		const failed = await store.bids.byCommitment(FAILED)
		expect(failed).toMatchObject({ success: false, error: "insufficient balance", extrinsicHash: null })
	})

	// The reason this file exists: a deposit is only reclaimable if its bid row
	// still comes back out of the operator's existing database.
	it("still finds the locked deposit a legacy bid row records", async () => {
		const { store } = openLegacyStore()

		expect((await store.bids.unretractedReclaimable()).map((bid) => bid.commitment)).toEqual([WON])
		expect((await store.bids.expiredUnretracted(60 * 60 * 1000)).map((bid) => bid.commitment)).toEqual([WON])
	})

	it("aggregates legacy rows without tripping over NULL sums", async () => {
		const { store } = openLegacyStore()

		expect(await store.bids.stats()).toEqual({
			total: 3,
			successful: 2,
			failed: 1,
			retracted: 1,
			pendingRetraction: 1,
		})
	})

	it("writes into a legacy bids.db and migrates it on disk", async () => {
		const { store, dir } = openLegacyStore()
		const fresh = `0x${"dd".repeat(32)}`

		await store.bids.store({
			commitment: fresh,
			success: true,
			pending: true,
			extrinsicHash: `0x${"ee".repeat(32)}`,
		})
		expect((await store.bids.byCommitment(fresh))?.pending).toBe(true)

		expect(await store.bids.markDead(WON)).toBe(true)
		expect((await store.bids.byCommitment(WON))?.dead).toBe(true)
		expect(await store.bids.markRetracted(WON, null)).toBe(true)
		expect((await store.bids.byCommitment(WON))?.retracted).toBe(true)
		// Already retracted, so the guarded UPDATE matches nothing the second time.
		expect(await store.bids.markRetracted(WON, null)).toBe(false)

		await store.close?.()
		const db = new DatabaseSync(join(dir, "bids.db"))
		const columns = (db.prepare("PRAGMA table_info(bids)").all() as unknown as Array<{ name: string }>).map(
			(c) => c.name,
		)
		db.close()
		expect(columns).toContain("dead")
		expect(columns).toContain("pending")
	})

	it("reads a WAL-mode activity.db that predates order_json", async () => {
		const { store, dir } = openLegacyStore()

		const events = await store.activity.recent()
		expect(events.map((e) => e.type)).toEqual(["lost", "filled", "bid"])
		expect(events.map((e) => e.order)).toEqual([null, null, null])
		expect(events.at(-1)).toMatchObject({
			ts: 1735786800000,
			type: "bid",
			orderId: "0xorder1",
			chainId: 1,
			strategy: "basic",
			success: true,
			volumeUsd: 250.5,
			profitUsd: 1.25,
			txHash: `0x${"66".repeat(32)}`,
		})
		expect(events[0]).toMatchObject({ type: "lost", reason: "outbid", success: false })

		// WAL is recorded in the file header, so an existing database keeps it.
		await store.close?.()
		const db = new DatabaseSync(join(dir, "activity.db"))
		const mode = (db.prepare("PRAGMA journal_mode").get() as unknown as { journal_mode: string }).journal_mode
		db.close()
		expect(mode).toBe("wal")
	})

	it("attaches an order summary to legacy rows through the explicit transaction", async () => {
		const { store } = openLegacyStore()

		expect(await store.activity.orderIdsMissingSummary()).toEqual(["0xorder2", "0xorder1"])

		const summary: OrderSummary = {
			user: `0x${"01".repeat(20)}`,
			source: "EVM-1",
			destination: "EVM-42161",
			placedTxHash: `0x${"02".repeat(32)}`,
			referrer: null,
			inputs: [{ token: `0x${"03".repeat(20)}`, amount: "1000000", symbol: "USDC", decimals: 6 }],
			outputs: [{ token: `0x${"04".repeat(20)}`, amount: "999000", symbol: "USDC", decimals: 6 }],
			deadline: "123456",
		}
		const updated = await store.activity.attachOrder("0xorder1", summary)

		// Both 0xorder1 rows lacked a summary, and both are returned.
		expect(updated.map((e) => e.type)).toEqual(["bid", "filled"])
		for (const event of updated) expect(event.order).toEqual(summary)
		// The untouched order is still listed as missing one.
		expect(await store.activity.orderIdsMissingSummary()).toEqual(["0xorder2"])
		// Re-running changes nothing, so nothing comes back.
		expect(await store.activity.attachOrder("0xorder1", summary)).toEqual([])
	})

	it("reads a legacy wallet ledger row and fills in the columns added later", async () => {
		const { store } = openLegacyStore()

		const [legacy] = await store.activity.walletTxs()
		expect(legacy).toMatchObject({
			ts: 1735787100000,
			kind: "sweep",
			chainId: 1,
			token: null,
			amount: null,
			to: null,
			txHash: `0x${"88".repeat(32)}`,
			sponsored: true,
			tokenIn: null,
			amountIn: null,
		})

		expect((await store.activity.walletTxsWithoutAmounts()).map((tx) => tx.id)).toEqual([legacy.id])
		await store.activity.updateWalletTx(legacy.id, {
			token: "USDC",
			amount: "12.5",
			to: `0x${"09".repeat(20)}`,
			tokenIn: "aUSDC",
			amountIn: "12.4",
		})
		expect((await store.activity.walletTxs())[0]).toMatchObject({
			token: "USDC",
			amount: "12.5",
			tokenIn: "aUSDC",
			amountIn: "12.4",
		})
	})

	it("reopens a directory it has already migrated without changing it again", async () => {
		const dir = legacyDataDir()

		const first = new SqliteDataStore(dir)
		await first.bids.store({ commitment: `0x${"ff".repeat(32)}`, success: true })
		await first.close?.()

		const second = new SqliteDataStore(dir)
		stores.push(second)
		expect((await second.bids.recent()).map((bid) => bid.commitment)).toEqual([
			`0x${"ff".repeat(32)}`,
			FAILED,
			RETRACTED,
			WON,
		])
		expect(await second.activity.knowsOrder("0xorder1")).toBe(true)
		expect(await second.activity.knowsOrder("0xnope")).toBe(false)
	})
})

describe("SqliteDataStore: a data directory it creates itself", () => {
	it("puts activity.db in WAL and leaves bids.db on the rollback journal", async () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-fresh-"))
		dirs.push(dir)

		const store = new SqliteDataStore(dir)
		stores.push(store)
		await store.activity.record({ type: "bid", orderId: "0xfresh" })
		await store.close?.()

		// Covers the `pragma("journal_mode = WAL")` → `exec(...)` conversion: the
		// legacy fixture already carries WAL in its header, so only a database this
		// store created from nothing can show that the call still does anything.
		const journalMode = (path: string) => {
			const db = new DatabaseSync(join(dir, path))
			const mode = (db.prepare("PRAGMA journal_mode").get() as unknown as { journal_mode: string }).journal_mode
			db.close()
			return mode
		}
		expect(journalMode("activity.db")).toBe("wal")
		expect(journalMode("bids.db")).toBe("delete")
	})

	it("creates a data directory that does not exist yet", async () => {
		const parent = mkdtempSync(join(tmpdir(), "simplex-parent-"))
		dirs.push(parent)
		const dir = join(parent, "nested", "simplex-data")

		const store = new SqliteDataStore(dir)
		stores.push(store)
		await store.bids.store({ commitment: `0x${"12".repeat(32)}`, success: true })
		expect((await store.bids.recent()).map((bid) => bid.commitment)).toEqual([`0x${"12".repeat(32)}`])
	})
})

describe("SqliteDataStore: lock contention", () => {
	// better-sqlite3 applied a 5s busy timeout by default; node:sqlite defaults to 0,
	// so a store that does not set one throws SQLITE_BUSY the instant anything else
	// holds the file — and a dropped bid write is a deposit nobody can reclaim.
	it("applies a busy timeout to both connections, whatever the runtime", async () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-busy-pragma-"))
		dirs.push(dir)
		const store = new SqliteDataStore(dir)
		stores.push(store)

		// Read it back off the store's OWN handles: busy_timeout is per-connection, so
		// a fresh handle would report the default no matter what the store did. This
		// asserts the mechanism directly, which the wall-clock test below cannot —
		// `DatabaseSync`'s `timeout` option is silently ignored before Node 22.18 and
		// on all of 23, so an option-based implementation passes here only by luck of
		// which runtime the suite happens to run on.
		// biome-ignore lint/suspicious/noExplicitAny: reaching past `private` for a per-connection pragma
		const connections = (store as any).databases as DatabaseSync[]
		expect(connections).toHaveLength(2)
		for (const db of connections) {
			const row = db.prepare("PRAGMA busy_timeout").get() as unknown as { timeout: number }
			expect(row.timeout).toBe(5000)
		}

		// A handle opened without the pragma is the contrast: SQLite's own default.
		const plain = new DatabaseSync(join(dir, "bids.db"))
		expect((plain.prepare("PRAGMA busy_timeout").get() as unknown as { timeout: number }).timeout).toBe(0)
		plain.close()
	})

	it("waits for another connection's write lock instead of failing instantly", async () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-busy-"))
		dirs.push(dir)
		const seed = new SqliteDataStore(dir)
		await seed.bids.store({ commitment: `0x${"31".repeat(32)}`, success: true })
		await seed.close?.()

		// Hold a write lock from outside, then prove the store's own handle waits.
		const holder = new DatabaseSync(join(dir, "bids.db"))
		holder.exec("BEGIN IMMEDIATE")
		holder.prepare("INSERT INTO bids (commitment, success) VALUES (?, 1)").run(`0x${"32".repeat(32)}`)

		const contended = new SqliteDataStore(dir)
		stores.push(contended)
		const startedAt = process.hrtime.bigint()
		await expect(contended.bids.store({ commitment: `0x${"33".repeat(32)}`, success: true })).rejects.toThrow(
			/locked|busy/i,
		)
		const waitedMs = Number(process.hrtime.bigint() - startedAt) / 1e6
		holder.exec("ROLLBACK")
		holder.close()

		// Without a busy timeout this returns in single-digit milliseconds.
		expect(waitedMs).toBeGreaterThan(1_000)
	}, 30_000)
})
