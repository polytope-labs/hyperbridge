import { afterEach, describe, expect, it } from "vitest"
import Database from "better-sqlite3"
import { mkdtempSync, rmSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import type { HexString } from "@hyperbridge/sdk"
import { BidStorageService } from "@/services/BidStorageService"

/**
 * Tests for the dead-bid bookkeeping introduced for #1074: a bid whose order was observed
 * filled on-chain is "dead" — it can never win, so the retraction sweep must pick it up on
 * its next cycle instead of waiting out the 1h stale-bid TTL. Also covers marking a bid
 * retracted without a retraction extrinsic hash (the BidNotFound-is-terminal path) and the
 * in-place schema migration for databases created before the `dead` column existed.
 */

const COMMITMENT = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const OTHER = "0x2222222222222222222222222222222222222222222222222222222222222222" as HexString
const HOUR_MS = 60 * 60 * 1000

describe("BidStorageService", () => {
	const dirs: string[] = []
	const services: BidStorageService[] = []

	function freshService(): { service: BidStorageService; dir: string } {
		const dir = mkdtempSync(join(tmpdir(), "simplex-bids-"))
		dirs.push(dir)
		const service = new BidStorageService(dir)
		services.push(service)
		return { service, dir }
	}

	/** Backdates a bid so it falls outside the given age, via a second connection to the same file. */
	function backdate(dir: string, commitment: HexString, ageMs: number): void {
		const db = new Database(join(dir, "bids.db"))
		const past = new Date(Date.now() - ageMs)
			.toISOString()
			.replace("T", " ")
			.replace(/\.\d{3}Z$/, "")
		db.prepare("UPDATE bids SET created_at = ? WHERE commitment = ?").run(past, commitment)
		db.close()
	}

	afterEach(() => {
		for (const service of services.splice(0)) {
			try {
				service.close()
			} catch {
				// already closed
			}
		}
		for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true })
	})

	it("stores bids with dead=false and exposes the flag on reads", () => {
		const { service } = freshService()
		service.storeBid({ commitment: COMMITMENT, success: true })

		const bid = service.getBidByCommitment(COMMITMENT)
		expect(bid).not.toBeNull()
		expect(bid!.dead).toBe(false)
		expect(bid!.retracted).toBe(false)
	})

	it("returns dead bids from getExpiredUnretractedBids regardless of age", () => {
		const { service } = freshService()
		service.storeBid({ commitment: COMMITMENT, success: true })
		service.storeBid({ commitment: OTHER, success: true })

		// Neither bid is older than the TTL, so only the dead one is due.
		expect(service.getExpiredUnretractedBids(HOUR_MS)).toHaveLength(0)

		expect(service.markBidAsDead(COMMITMENT)).toBe(true)

		const due = service.getExpiredUnretractedBids(HOUR_MS)
		expect(due.map((bid) => bid.commitment)).toEqual([COMMITMENT])
		expect(due[0].dead).toBe(true)
	})

	it("still returns stale bids past the TTL even when not dead", () => {
		const { service, dir } = freshService()
		service.storeBid({ commitment: COMMITMENT, success: true })
		backdate(dir, COMMITMENT, 2 * HOUR_MS)

		const due = service.getExpiredUnretractedBids(HOUR_MS)
		expect(due.map((bid) => bid.commitment)).toEqual([COMMITMENT])
		expect(due[0].dead).toBe(false)
	})

	it("never returns failed or retracted bids, dead or not", () => {
		const { service } = freshService()
		service.storeBid({ commitment: COMMITMENT, success: false, error: "bid submission failed" })
		service.markBidAsDead(COMMITMENT)
		service.storeBid({ commitment: OTHER, success: true })
		service.markBidAsDead(OTHER)
		service.markBidAsRetracted(OTHER, "0xretract" as HexString)

		expect(service.getExpiredUnretractedBids(HOUR_MS)).toHaveLength(0)
	})

	it("marks a bid retracted with a null extrinsic hash (BidNotFound: nothing left to reclaim)", () => {
		const { service } = freshService()
		service.storeBid({ commitment: COMMITMENT, success: true })

		expect(service.markBidAsRetracted(COMMITMENT, null)).toBe(true)

		const bid = service.getBidByCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(true)
		expect(bid!.retractExtrinsicHash).toBeNull()

		// Terminal: neither re-retraction nor death apply afterwards.
		expect(service.markBidAsRetracted(COMMITMENT, null)).toBe(false)
		expect(service.markBidAsDead(COMMITMENT)).toBe(false)
	})

	it("adds the dead column in place to databases created before it existed", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-bids-legacy-"))
		dirs.push(dir)

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

		const service = new BidStorageService(dir)
		services.push(service)

		const bid = service.getBidByCommitment(COMMITMENT)
		expect(bid).not.toBeNull()
		expect(bid!.dead).toBe(false)

		expect(service.markBidAsDead(COMMITMENT)).toBe(true)
		expect(service.getExpiredUnretractedBids(HOUR_MS).map((b) => b.commitment)).toEqual([COMMITMENT])
	})
})
