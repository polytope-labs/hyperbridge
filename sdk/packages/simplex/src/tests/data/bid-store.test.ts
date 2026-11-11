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
 * The bid store's contract, against BOTH shipped implementations. The in-memory store is what
 * `Simplex.start` uses by default, so a consumer who never configures persistence must still get
 * identical semantics: a divergence there would mis-attribute fills.
 *
 * A bid is placed when the orderbook holds it. Rows written before 0.18.6, when simplex also bid
 * on Hyperbridge, keep their Hyperbridge history, which nothing writes any more.
 */

const COMMITMENT = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const OTHER = "0x2222222222222222222222222222222222222222222222222222222222222222" as HexString
const THIRD = "0x3333333333333333333333333333333333333333333333333333333333333333" as HexString

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

const backends: Array<{ name: string; create: () => BidStore }> = [
	{
		name: "MemoryDataStore",
		create: () => {
			const store = new MemoryDataStore()
			stores.push(store)
			return store.bids
		},
	},
	{
		name: "SqliteDataStore",
		create: () => {
			const store = new SqliteDataStore(tempDir("simplex-bids-"))
			stores.push(store)
			return store.bids
		},
	},
]

for (const backend of backends) {
	describe(`BidStore: ${backend.name}`, () => {
		it("stores a bid the orderbook holds as placed, with no Hyperbridge history", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, bid: `0x${"b1".repeat(32)}`, success: true, onOrderbook: true })

			expect(await bids.byCommitment(COMMITMENT)).toMatchObject({
				commitment: COMMITMENT,
				success: true,
				onOrderbook: true,
				error: null,
				pending: false,
				extrinsicHash: null,
				blockHash: null,
				retracted: false,
				retractedAt: null,
				retractExtrinsicHash: null,
				dead: false,
				pricedBy: null,
			})
		})

		it("keeps the identifier each bid was placed under", async () => {
			// What tells the bids on one order apart, and names the one a fill is attributed to.
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, bid: `0x${"b1".repeat(32)}`, success: true })
			await bids.store({ commitment: OTHER, success: true })

			expect((await bids.byCommitment(COMMITMENT))!.bid).toBe(`0x${"b1".repeat(32)}`)
			expect((await bids.byCommitment(OTHER))!.bid).toBeNull()
		})

		it("counts a bid placed when a venue took it, and failed when none did", async () => {
			const bids = backend.create()
			await bids.store({ commitment: COMMITMENT, success: true, onOrderbook: true })
			await bids.store({ commitment: OTHER, success: false, error: "NO_LIMIT_ORDER: no limit order" })
			await bids.store({ commitment: THIRD, success: true, onOrderbook: true })

			expect(await bids.stats()).toEqual({ total: 3, successful: 2, failed: 1 })
			expect((await bids.failed()).map((bid) => [bid.commitment, bid.error])).toEqual([
				[OTHER, "NO_LIMIT_ORDER: no limit order"],
			])
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
			// Provenance, not a hold: it is written once and read back as it was.
			const bids = backend.create()
			const first = `0x${"b1".repeat(32)}`
			const second = `0x${"b2".repeat(32)}`
			const pricedBy = { limitOrderId: "limit-1", payout: "100", take: "7", takeDecimals: 6 }
			await bids.store({ commitment: COMMITMENT, bid: first, success: true, pricedBy })
			await bids.store({ commitment: COMMITMENT, bid: second, success: true, pricedBy: { limitOrderId: "limit-2", payout: "200" } })

			const rows = await bids.byCommitments([COMMITMENT])
			expect(rows.map((bid) => [bid.bid, bid.pricedBy])).toEqual([
				[second, { limitOrderId: "limit-2", payout: "200" }],
				[first, pricedBy],
			])
			expect((await bids.byLimitOrder("limit-1")).map((bid) => bid.bid)).toEqual([first])
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
		expect(bid!.onOrderbook).toBe(false)
		expect(await store.bids.stats()).toEqual({ total: 1, successful: 1, failed: 0 })
	})

	it("reads a bid Hyperbridge took before 0.18.6 with its history, and writes beside it", async () => {
		const dir = tempDir("simplex-bids-hyperbridge-")
		const first = new SqliteDataStore(dir)
		await first.bids.store({ commitment: COMMITMENT, success: true })
		await first.close()

		// The row as a release that bid on Hyperbridge left it: placed, then retracted.
		const db = new DatabaseSync(join(dir, "bids.db"))
		db.prepare(
			"UPDATE bids SET extrinsic_hash = '0xplace', retracted = 1, retracted_at = datetime('now'), retract_extrinsic_hash = '0xretract', dead = 1",
		).run()
		db.close()

		const store = new SqliteDataStore(dir)
		stores.push(store)
		expect(await store.bids.byCommitment(COMMITMENT)).toMatchObject({
			success: true,
			onOrderbook: false,
			extrinsicHash: "0xplace",
			retracted: true,
			retractExtrinsicHash: "0xretract",
			dead: true,
		})

		await store.bids.store({ commitment: OTHER, success: true, onOrderbook: true })
		expect(await store.bids.byCommitment(OTHER)).toMatchObject({ extrinsicHash: null, retracted: false, dead: false })
		expect(await store.bids.stats()).toEqual({ total: 2, successful: 2, failed: 0 })
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
