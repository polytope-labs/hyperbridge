import { afterEach, describe, expect, it } from "vitest"
import Database from "better-sqlite3"
import { mkdtempSync, rmSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import type { HexString } from "@hyperbridge/sdk"
import { MemoryDataStore } from "@/data/memory"
import { SqliteDataStore } from "@/data/sqlite"
import type { BidStore, SimplexDataStore } from "@/data/types"

/**
 * Dead-bid bookkeeping (#1074): a bid whose order was observed filled on-chain is
 * "dead" — it can never win, so the retraction sweep must pick it up on its next
 * cycle instead of waiting out the 1h stale-bid TTL. Also covers marking a bid
 * retracted without a retraction extrinsic hash (the BidNotFound-is-terminal path).
 *
 * The behavioural suite runs against BOTH shipped implementations. The in-memory
 * store is what `Simplex.start` uses by default, so a consumer who never
 * configures persistence must still get identical retraction semantics — a
 * divergence there would strand deposits rather than just lose history.
 */

const COMMITMENT = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const OTHER = "0x2222222222222222222222222222222222222222222222222222222222222222" as HexString
const HOUR_MS = 60 * 60 * 1000

const dirs: string[] = []
const stores: SimplexDataStore[] = []

function tempDir(prefix: string): string {
	const dir = mkdtempSync(join(tmpdir(), prefix))
	dirs.push(dir)
	return dir
}

afterEach(async () => {
	for (const store of stores.splice(0)) {
		try {
			await store.close?.()
		} catch {
			// already closed
		}
	}
	for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true })
})

/** Backdates a bid past the given age. Each backend needs its own reach-around. */
type Backdate = (bids: BidStore, commitment: HexString, ageMs: number) => Promise<void>

const backends: Array<{ name: string; create: () => BidStore; backdate: Backdate }> = [
	{
		name: "MemoryDataStore",
		create: () => {
			const store = new MemoryDataStore()
			stores.push(store)
			return store.bids
		},
		// The memory store has no clock seam, so re-store the bid with the row
		// mutated in place through the only handle a test legitimately has.
		backdate: async (bids, commitment, ageMs) => {
			const rows = await bids.unretractedSuccessful()
			const row = rows.find((r) => r.commitment === commitment)
			if (!row) throw new Error("bid not found")
			const past = new Date(Date.now() - ageMs)
				.toISOString()
				.replace("T", " ")
				.replace(/\.\d{3}Z$/, "")
			// biome-ignore lint/suspicious/noExplicitAny: reaching into the store's row for a clock test
			const internal = (bids as any).rows.find((r: { commitment: string }) => r.commitment === commitment)
			internal.createdAt = past
		},
	},
	{
		name: "SqliteDataStore",
		create: () => {
			const dir = tempDir("simplex-bids-")
			const store = new SqliteDataStore(dir)
			stores.push(store)
			// Stash the directory on the store so backdate can reopen the file.
			// biome-ignore lint/suspicious/noExplicitAny: test-only handle
			;(store.bids as any).__dir = dir
			return store.bids
		},
		backdate: async (bids, commitment, ageMs) => {
			// biome-ignore lint/suspicious/noExplicitAny: test-only handle
			const dir = (bids as any).__dir as string
			const db = new Database(join(dir, "bids.db"))
			const past = new Date(Date.now() - ageMs)
				.toISOString()
				.replace("T", " ")
				.replace(/\.\d{3}Z$/, "")
			db.prepare("UPDATE bids SET created_at = ? WHERE commitment = ?").run(past, commitment)
			db.close()
		},
	},
]

for (const backend of backends) {
	describe(`BidStore: ${backend.name}`, () => {
		it("stores bids with dead=false and exposes the flag on reads", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: true })

			const bid = await bids.byCommitment(COMMITMENT)
			expect(bid).not.toBeNull()
			expect(bid!.dead).toBe(false)
			expect(bid!.retracted).toBe(false)
		})

		it("returns dead bids from expiredUnretracted regardless of age", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: true })
			await bids.store({ commitment: OTHER, success: true })

			// Neither bid is older than the TTL, so only the dead one is due.
			expect(await bids.expiredUnretracted(HOUR_MS)).toHaveLength(0)

			expect(await bids.markDead(COMMITMENT)).toBe(true)

			const due = await bids.expiredUnretracted(HOUR_MS)
			expect(due.map((bid) => bid.commitment)).toEqual([COMMITMENT])
			expect(due[0].dead).toBe(true)
		})

		it("still returns stale bids past the TTL even when not dead", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: true })
			await backend.backdate(bids, COMMITMENT, 2 * HOUR_MS)

			const due = await bids.expiredUnretracted(HOUR_MS)
			expect(due.map((bid) => bid.commitment)).toEqual([COMMITMENT])
			expect(due[0].dead).toBe(false)
		})

		it("never returns failed or retracted bids, dead or not", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: false, error: "bid submission failed" })
			await bids.markDead(COMMITMENT)
			await bids.store({ commitment: OTHER, success: true })
			await bids.markDead(OTHER)
			await bids.markRetracted(OTHER, "0xretract" as HexString)

			expect(await bids.expiredUnretracted(HOUR_MS)).toHaveLength(0)
		})

		it("marks a bid retracted with a null extrinsic hash (BidNotFound: nothing left to reclaim)", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: true })

			expect(await bids.markRetracted(COMMITMENT, null)).toBe(true)

			const bid = await bids.byCommitment(COMMITMENT)
			expect(bid!.retracted).toBe(true)
			expect(bid!.retractExtrinsicHash).toBeNull()

			// Terminal: neither re-retraction nor death apply afterwards.
			expect(await bids.markRetracted(COMMITMENT, null)).toBe(false)
			expect(await bids.markDead(COMMITMENT)).toBe(false)
		})

		it("counts bids by state", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: true })
			await bids.store({ commitment: OTHER, success: false, error: "nope" })
			await bids.markRetracted(COMMITMENT, null)

			expect(await bids.stats()).toEqual({
				total: 2,
				successful: 1,
				failed: 1,
				retracted: 1,
				pendingRetraction: 0,
			})
		})
	})
}

describe("SqliteBidStore migrations", () => {
	it("adds the dead column in place to databases created before it existed", async () => {
		const dir = tempDir("simplex-bids-legacy-")

		// The exact pre-#1074 schema, with a row already in it.
		const legacy = new Database(join(dir, "bids.db"))
		legacy.exec(`
			CREATE TABLE bids (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				commitment TEXT NOT NULL,
				extrinsic_hash TEXT,
				block_hash TEXT,
				success INTEGER NOT NULL,
				error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				retracted INTEGER NOT NULL DEFAULT 0,
				retracted_at TEXT,
				retract_extrinsic_hash TEXT
			);
		`)
		legacy.prepare("INSERT INTO bids (commitment, success) VALUES (?, 1)").run(COMMITMENT)
		legacy.close()

		const store = new SqliteDataStore(dir)
		stores.push(store)

		const bid = await store.bids.byCommitment(COMMITMENT)
		expect(bid).not.toBeNull()
		expect(bid!.dead).toBe(false)

		expect(await store.bids.markDead(COMMITMENT)).toBe(true)
		expect((await store.bids.expiredUnretracted(HOUR_MS)).map((b) => b.commitment)).toEqual([COMMITMENT])
	})
})
