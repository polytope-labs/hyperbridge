import { afterEach, describe, expect, it } from "vitest"
import { DatabaseSync } from "node:sqlite"
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
const THIRD = "0x3333333333333333333333333333333333333333333333333333333333333333" as HexString
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
			const rows = await bids.unretractedReclaimable()
			const row = rows.find((r: { commitment: string }) => r.commitment === commitment)
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
			const db = new DatabaseSync(join(dir, "bids.db"))
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

		it("keeps the identifier each bid was placed under", async () => {
			// What retracting the bid names; recorded at placement, never looked up.
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, bid: `0x${"b1".repeat(32)}`, success: true })
			await bids.store({ commitment: OTHER, success: true })

			expect((await bids.byCommitment(COMMITMENT))!.bid).toBe(`0x${"b1".repeat(32)}`)
			expect((await bids.byCommitment(OTHER))!.bid).toBeNull()
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

		it("finds the bids a limit order priced, newest first", async () => {
			// What makes a limit order's `remaining` explicable: which bids drew on it.
			const bids = backend.create()
			const priced = (limitOrderId: string, payout: string) => ({ limitOrderId, payout })
			await bids.store({ commitment: COMMITMENT, success: true, pricedBy: priced("limit-1", "100") })
			await bids.store({ commitment: OTHER, success: true, pricedBy: priced("limit-2", "200") })
			await bids.store({ commitment: THIRD, success: true, pricedBy: priced("limit-1", "300") })
			// No limit order priced this one.
			await bids.store({ commitment: THIRD, success: true })

			const drew = await bids.byLimitOrder("limit-1")
			expect(drew.map((bid) => bid.commitment)).toEqual([THIRD, COMMITMENT])
			expect(drew.map((bid) => bid.pricedBy?.payout)).toEqual(["300", "100"])
			expect((await bids.byLimitOrder("limit-2")).map((bid) => bid.commitment)).toEqual([OTHER])
			expect(await bids.byLimitOrder("limit-3")).toEqual([])
		})

		it("keeps what priced a bid as written, whatever happens to the bid after", async () => {
			// Provenance, not a hold: retracting or killing the bid does not clear it.
			const bids = backend.create()
			const first = `0x${"b1".repeat(32)}`
			const second = `0x${"b2".repeat(32)}`
			const pricedBy = { limitOrderId: "limit-1", payout: "100", take: "7", takeDecimals: 6 }
			await bids.store({ commitment: COMMITMENT, bid: first, success: true, pricedBy })
			await bids.store({ commitment: COMMITMENT, bid: second, success: true, pricedBy: { limitOrderId: "limit-2", payout: "200" } })

			await bids.markDead(COMMITMENT)
			await bids.markRetracted(COMMITMENT, null)

			const rows = await bids.byCommitments([COMMITMENT])
			expect(rows.map((bid) => [bid.bid, bid.pricedBy])).toEqual([
				[second, { limitOrderId: "limit-2", payout: "200" }],
				[first, pricedBy],
			])
			expect((await bids.byLimitOrder("limit-1")).map((bid) => bid.bid)).toEqual([first])
		})

		it("marks retracted and flags dead only the rows a retraction names", async () => {
			// A retraction closes the rows it read and sent retractions for, not a bid stored
			// on the commitment while it was in flight.
			const bids = backend.create()
			const first = `0x${"b1".repeat(32)}`
			const second = `0x${"b2".repeat(32)}`
			await bids.store({ commitment: COMMITMENT, bid: first, success: true })
			await bids.store({ commitment: COMMITMENT, bid: second, success: true })
			await bids.store({ commitment: OTHER, success: true })
			const firstRow = (await bids.byCommitments([COMMITMENT])).find((row) => row.bid === first)!
			const secondRow = (await bids.byCommitments([COMMITMENT])).find((row) => row.bid === second)!
			const other = (await bids.byCommitment(OTHER))!

			// Another commitment's row id names nothing here, and an empty list names nothing.
			expect(await bids.markRetracted(COMMITMENT, "0xr", [other.id])).toBe(false)
			expect(await bids.markRetracted(COMMITMENT, "0xr", [])).toBe(false)
			expect(await bids.markDead(COMMITMENT, [])).toBe(false)
			expect(await bids.markDead(COMMITMENT, [other.id])).toBe(false)

			expect(await bids.markRetracted(COMMITMENT, "0xr", [firstRow.id])).toBe(true)
			expect(
				(await bids.byCommitments([COMMITMENT])).map((row) => [row.bid, row.retracted, row.retractExtrinsicHash]).sort(),
			).toEqual([
				[first, true, "0xr"],
				[second, false, null],
			])
			expect((await bids.byCommitment(OTHER))!.retracted).toBe(false)

			// Flagging dead by row leaves the commitment's other bids alone.
			expect(await bids.markDead(COMMITMENT, [secondRow.id])).toBe(true)
			expect((await bids.byCommitments([COMMITMENT])).map((row) => [row.bid, row.dead]).sort()).toEqual([
				[first, false],
				[second, true],
			])
			expect((await bids.byCommitment(OTHER))!.dead).toBe(false)
			// A retracted row is not flagged again.
			expect(await bids.markDead(COMMITMENT, [firstRow.id])).toBe(false)
		})

		it("marks the whole commitment retracted and dead when no rows are named", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, bid: `0x${"b1".repeat(32)}`, success: true })
			await bids.store({ commitment: COMMITMENT, bid: `0x${"b2".repeat(32)}`, success: true })
			await bids.store({ commitment: OTHER, success: true })

			expect(await bids.markDead(COMMITMENT)).toBe(true)
			expect((await bids.byCommitments([COMMITMENT])).every((row) => row.dead)).toBe(true)
			expect((await bids.byCommitment(OTHER))!.dead).toBe(false)

			expect(await bids.markRetracted(COMMITMENT, null)).toBe(true)
			expect((await bids.byCommitments([COMMITMENT])).every((row) => row.retracted)).toBe(true)
			expect((await bids.byCommitment(OTHER))!.retracted).toBe(false)
		})
	})
}

describe("SqliteBidStore migrations", () => {
	it("adds the dead column in place to databases created before it existed", async () => {
		const dir = tempDir("simplex-bids-legacy-")

		// The exact pre-#1074 schema, with a row already in it.
		const legacy = new DatabaseSync(join(dir, "bids.db"))
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
		// A row from before bids carried an identifier has none.
		expect(bid!.bid).toBeNull()

		expect(await store.bids.markDead(COMMITMENT)).toBe(true)
		expect((await store.bids.expiredUnretracted(HOUR_MS)).map((b) => b.commitment)).toEqual([COMMITMENT])
	})
})

describe("SqliteBidStore provenance across the upgrade", () => {
	/** A database as the build with limit-order holds left it: `reservations` and no `priced_by`. */
	function holdsEraDatabase(dir: string) {
		const legacy = new DatabaseSync(join(dir, "bids.db"))
		legacy.exec(`
			CREATE TABLE bids (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				commitment TEXT NOT NULL,
				bid TEXT,
				extrinsic_hash TEXT,
				block_hash TEXT,
				success INTEGER NOT NULL,
				pending INTEGER NOT NULL DEFAULT 0,
				error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				retracted INTEGER NOT NULL DEFAULT 0,
				retracted_at TEXT,
				retract_extrinsic_hash TEXT,
				dead INTEGER NOT NULL DEFAULT 0,
				reservations TEXT
			);
		`)
		return legacy
	}

	it("adds priced_by in place and reads a bid placed before it from its reservations", async () => {
		const dir = tempDir("simplex-bids-holds-")
		const legacy = holdsEraDatabase(dir)
		const hold = [{ limitOrderId: "limit-1", amount: "1000", take: "100", takeDecimals: 6 }]
		legacy.prepare("INSERT INTO bids (commitment, bid, success, reservations) VALUES (?, ?, 1, ?)").run(
			COMMITMENT,
			"0xb1",
			JSON.stringify(hold),
		)
		// A bid whose hold that build had already settled has nothing left to read.
		legacy.prepare("INSERT INTO bids (commitment, bid, success) VALUES (?, ?, 1)").run(OTHER, "0xb2")
		legacy.close()

		const store = new SqliteDataStore(dir)
		stores.push(store)

		const [bid] = await store.bids.byCommitments([COMMITMENT])
		expect(bid.pricedBy).toEqual({ limitOrderId: "limit-1", payout: "1000", take: "100", takeDecimals: 6 })
		expect(bid).not.toHaveProperty("reservations")
		expect((await store.bids.byCommitment(OTHER))!.pricedBy).toBeNull()
		expect((await store.bids.byLimitOrder("limit-1")).map((row) => row.commitment)).toEqual([COMMITMENT])
	})

	it("writes priced_by and leaves reservations untouched, so an older build still opens the file", async () => {
		const dir = tempDir("simplex-bids-rollback-")
		holdsEraDatabase(dir).close()

		const store = new SqliteDataStore(dir)
		stores.push(store)
		await store.bids.store({ commitment: COMMITMENT, success: true, pricedBy: { limitOrderId: "limit-1", payout: "5" } })
		await store.close()

		const raw = new DatabaseSync(join(dir, "bids.db"))
		const row = raw.prepare("SELECT reservations, priced_by FROM bids WHERE commitment = ?").get(COMMITMENT) as {
			reservations: string | null
			priced_by: string | null
		}
		raw.close()
		expect(row.reservations).toBeNull()
		expect(JSON.parse(row.priced_by!)).toEqual({ limitOrderId: "limit-1", payout: "5" })
	})
})
