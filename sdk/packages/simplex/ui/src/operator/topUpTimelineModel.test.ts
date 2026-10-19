import { describe, expect, it } from "vitest"
import {
	formatDisplayAmount,
	formatStepDuration,
	offerCount,
	selectedSolver,
	RELAYER_DELIVERY_WAIT_MS,
	type TimelineTopUp,
	timelineOffers,
	timelineSteps,
} from "./topUpTimelineModel"

const SOLVER = "0xce319986ca4D5d0893751a628D0db3Dc8fc91d62"
const OTHER = "0x1111111111111111111111111111111111111111"

function topUp(patch: Partial<TimelineTopUp>): TimelineTopUp {
	return { chain: "EVM-8453", token: "USDC", stage: "placing", createdAt: 0, updatedAt: 0, ...patch }
}

const statuses = (t: TimelineTopUp) => timelineSteps(t, "Base").map((step) => [step.key, step.caption, step.status])

describe("top-up timeline", () => {
	it("shows every step from the start, the later ones upcoming", () => {
		expect(statuses(topUp({ stage: "placing" }))).toEqual([
			["placed", "Placing Order", "processing"],
			["offers", "Awaiting Offers", "upcoming"],
			["fulfilled", "Filling Order", "upcoming"],
			["finalized", "Transaction Finalized", "upcoming"],
			["verified", "Transaction Complete", "upcoming"],
		])
	})

	it("follows the swap, then the BRIDGE's trip into Hyperbridge", () => {
		expect(statuses(topUp({ stage: "awaiting_bids", stageTimes: { awaiting_bids: 1 } }))).toEqual([
			["placed", "Order Placed", "completed"],
			["offers", "Awaiting Offers", "processing"],
			["fulfilled", "Filling Order", "upcoming"],
			["finalized", "Transaction Finalized", "upcoming"],
			["verified", "Transaction Complete", "upcoming"],
		])
		expect(
			statuses(
				topUp({
					stage: "delivering",
					stageTimes: { awaiting_bids: 1, delivering: 40_000 },
					deliveryStatuses: [{ status: "SOURCE", timestamp: 40_000, txHash: "0xaa" }],
				}),
			),
		).toEqual([
			["placed", "Order Placed", "completed"],
			["offers", "Offers Received", "completed"],
			["fulfilled", "Order Fulfilled", "completed"],
			["finalized", "Transaction Finalized", "processing"],
			["verified", "Transaction Complete", "upcoming"],
		])
	})

	it("counts each step down from the previous one's end and times it once done", () => {
		const steps = timelineSteps(
			topUp({
				stage: "delivering",
				stageTimes: { awaiting_bids: 10_000 },
				events: [{ status: "BIDS_RECEIVED", timestamp: 37_000, bidCount: 1 }],
				deliveryStatuses: [
					{ status: "SOURCE", timestamp: 38_000, txHash: "0xfill" },
					{ status: "SOURCE_FINALIZED", timestamp: 158_000, txHash: "0xfinal" },
				],
			}),
			"Base",
		)
		const byKey = Object.fromEntries(steps.map((step) => [step.key, step]))
		expect(formatStepDuration(byKey.offers.duration!)).toBe("27 seconds")
		expect(formatStepDuration(byKey.finalized.duration!)).toBe("2 minutes")
		// Hyperbridge verification is expected 3.5 minutes after BNB Chain is final.
		expect(byKey.verified.status).toBe("processing")
		expect(byKey.verified.etaAt).toBe(158_000 + 210_000)
		expect(byKey.finalized.link).toEqual({ chain: "hyperbridge", hash: "0xfinal" })
	})

	it("lists each offer once, with the selected solver from the fill", () => {
		const t = topUp({
			stage: "delivering",
			filler: SOLVER,
			events: [
				{ status: "NEW_BID", timestamp: 5, bids: [{ solverAddress: SOLVER, outputs: [{ token: "0x", amount: "1" }] }] },
				{
					status: "BIDS_RECEIVED",
					timestamp: 6,
					bidCount: 2,
					bids: [
						{ solverAddress: SOLVER, outputs: [{ token: "0x", amount: "1" }] },
						{ solverAddress: OTHER, outputs: [{ token: "0x", amount: "2" }] },
					],
				},
			],
		})
		expect(timelineOffers(t).map((offer) => offer.preview.solverAddress)).toEqual([SOLVER, OTHER])
		expect(offerCount(t)).toBe(2)
		expect(selectedSolver(t)).toBe(SOLVER)
	})

	it("links the completed delivery to its message until the indexer has its transaction", () => {
		const delivered = (statuses: TimelineTopUp["deliveryStatuses"]) =>
			timelineSteps(topUp({ stage: "delivered", deliveryCommitment: "0xcommit", deliveryStatuses: statuses }), "Base").find(
				(step) => step.key === "verified",
			)!.link
		expect(delivered([])).toEqual({ chain: "message", hash: "0xcommit" })
		expect(delivered([{ status: "DESTINATION", timestamp: 1, txHash: "0xdest" }])).toEqual({
			chain: "hyperbridge",
			hash: "0xdest",
		})
	})

	it("expires a cross-chain order before recovery as the order's steps, ending in Order Expired", () => {
		expect(statuses(topUp({ stage: "unfilled", stageTimes: { awaiting_bids: 1 } }))).toEqual([
			["placed", "Order Placed", "completed"],
			["offers", "Awaiting Offers", "upcoming"],
			["fulfilled", "Order Expired", "error"],
		])
	})

	it("expires a same-chain order into a red Claim Funds, then Funds Claimed", () => {
		const sameChain = (patch: Partial<TimelineTopUp>) =>
			statuses(topUp({ chain: "EVM-56", stageTimes: { awaiting_bids: 1 }, ...patch }))
		expect(sameChain({ stage: "unfilled" })).toEqual([
			["placed", "Order Placed", "completed"],
			["FUNDS_CLAIMED", "Claim Funds", "error"],
		])
		expect(
			sameChain({ stage: "reclaimed", recoverySteps: [{ step: "FUNDS_CLAIMED", timestamp: 2, txHash: "0xrefund" }] }),
		).toEqual([
			["placed", "Order Placed", "completed"],
			["FUNDS_CLAIMED", "Funds Claimed", "completed"],
		])
	})

	it("recovers a cross-chain order step by step, as the Hyperbridge app does", () => {
		const startedAt = 1_000_000
		const recovering = topUp({
			stage: "reclaiming",
			stageTimes: { awaiting_bids: 1, reclaiming: startedAt },
			reclaimTxHash: "0xcancel",
			recoverySteps: [
				{ step: "DESTINATION_FINALIZED", timestamp: startedAt + 170_000 },
				{ step: "ORDER_CANCELED", timestamp: startedAt + 190_000, txHash: "0xcancel" },
			],
		})
		const steps = timelineSteps(recovering, "Base", startedAt + 200_000)
		expect(steps.map((step) => [step.key, step.caption, step.status, step.level])).toEqual([
			["DESTINATION_FINALIZED", "BNB Chain Finalized", "completed", 0],
			["ORDER_CANCELED", "Order Canceled", "completed", 1],
			["SOURCE_FINALIZED", "Awaiting Base Finality", "processing", 1],
			["HYPERBRIDGE_DELIVERED", "Hyperbridge Verifying", "upcoming", 1],
			["HYPERBRIDGE_FINALIZED", "Hyperbridge Finalizing", "upcoming", 1],
			["FUNDS_CLAIMED", "Claim Funds", "upcoming", 0],
		])
		const byKey = Object.fromEntries(steps.map((step) => [step.key, step]))
		expect(byKey.DESTINATION_FINALIZED.duration).toBe(170_000)
		// Base finalizes in about an hour and a half, counted from the cancel.
		expect(byKey.SOURCE_FINALIZED.etaAt).toBe(startedAt + 190_000 + 90 * 60_000)
		expect(byKey.ORDER_CANCELED.link).toEqual({ chain: "source", hash: "0xcancel" })
	})

	it("keeps a stopped recovery's steps and marks the one it stopped at, with the reason and how to resume", () => {
		const stopped = topUp({
			stage: "unfilled",
			stageTimes: { awaiting_bids: 1, reclaiming: 1_000, unfilled: 9_000 },
			error: "Recovery stopped after 6 failed attempts: Timed out reading the indexer",
			recoverySteps: [
				{ step: "DESTINATION_FINALIZED", timestamp: 2_000 },
				{ step: "ORDER_CANCELED", timestamp: 3_000, txHash: "0xcancel" },
			],
		})
		const steps = timelineSteps(stopped, "Base", 10_000)
		expect(steps.map((step) => [step.key, step.status])).toEqual([
			["DESTINATION_FINALIZED", "completed"],
			["ORDER_CANCELED", "completed"],
			["SOURCE_FINALIZED", "error"],
			["HYPERBRIDGE_DELIVERED", "upcoming"],
			["HYPERBRIDGE_FINALIZED", "upcoming"],
			["FUNDS_CLAIMED", "upcoming"],
		])
		const failed = steps[2]
		expect(failed.secondary).toBe("Recovery stopped: Timed out reading the indexer — Start recovery to resume")
		// A stopped step no longer counts down.
		expect(failed.timed).toBe(false)
		expect(failed.etaAt).toBeUndefined()
		expect(steps[1].link).toEqual({ chain: "source", hash: "0xcancel" })
	})

	it("waits for a relayer to deliver the refund, then claims it", () => {
		const finalizedAt = 5_000_000
		const recovering = (now: number) =>
			timelineSteps(
				topUp({
					stage: "reclaiming",
					stageTimes: { reclaiming: 1 },
					recoverySteps: [
						{ step: "DESTINATION_FINALIZED", timestamp: 2 },
						{ step: "ORDER_CANCELED", timestamp: 3 },
						{ step: "SOURCE_FINALIZED", timestamp: 4 },
						{ step: "HYPERBRIDGE_DELIVERED", timestamp: 5 },
						{ step: "HYPERBRIDGE_FINALIZED", timestamp: finalizedAt },
					],
				}),
				"Base",
				now,
			).at(-1)!
		expect(recovering(finalizedAt + 60_000).caption).toBe("Waiting for Relayer")
		expect(recovering(finalizedAt + RELAYER_DELIVERY_WAIT_MS + 1).caption).toBe("Claim Funds")
	})

	it("formats amounts as the Hyperbridge app does", () => {
		expect(formatDisplayAmount("1979010000000000000", 18)).toBe("1.979")
		expect(formatDisplayAmount("123456789", 18)).toBe("0.0000000001234")
	})
})
