import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { OrderbookBidPlacer, bidRefusal, orderbookHoldsBid } from "@/core/bid-placer"
import type { SubmitBidResult } from "@/orderbook/types"

/** Every bid goes to the HyperFX orderbook, and only there. */

const COMMITMENT = `0x${"c0".repeat(32)}` as HexString
const USER_OP = "0x0102" as HexString

const ACCEPTED: SubmitBidResult = {
	kind: "accepted",
	bid: {
		commitment: COMMITMENT,
		leg: 0,
		side: "BID",
		fillChain: "EVM-97",
		sourceChain: "EVM-97",
		price: "1500",
		amountIn: "1",
		amountOut: "1500",
		expiresAt: "2026-10-21T00:10:00Z",
	},
}
const REJECTED: SubmitBidResult = { kind: "rejected", code: "NO_LIMIT_ORDER", message: "no limit order on the book" }
const FAILED: SubmitBidResult = { kind: "failed", code: "TIMEOUT", message: "try again", retryable: true }

const logger = { trace: vi.fn(), debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn(), fatal: vi.fn() }

function placerAnswering(answer: SubmitBidResult | Error) {
	const submitBid = vi.fn(async () => {
		if (answer instanceof Error) throw answer
		return answer
	})
	return { placer: new OrderbookBidPlacer({ submitBid }, logger as never), submitBid }
}

describe("OrderbookBidPlacer", () => {
	it("posts the signed op, SCALE-encoded as the strategy built it, and returns the orderbook's answer", async () => {
		const { placer, submitBid } = placerAnswering(ACCEPTED)

		expect(await placer.place(COMMITMENT, USER_OP)).toBe(ACCEPTED)
		expect(submitBid).toHaveBeenCalledExactlyOnceWith(USER_OP)
	})

	it("passes the orderbook's refusal through and logs its code", async () => {
		const { placer } = placerAnswering(REJECTED)
		logger.info.mockClear()

		expect(await placer.place(COMMITMENT, USER_OP)).toBe(REJECTED)
		expect(logger.info).toHaveBeenCalledWith(
			expect.objectContaining({ commitment: COMMITMENT, code: "NO_LIMIT_ORDER" }),
			"The orderbook refused the bid",
		)
	})

	it("renders a request that got no answer as a failed submission rather than rejecting", async () => {
		const { placer } = placerAnswering(new Error("Could not reach the orderbook"))

		expect(await placer.place(COMMITMENT, USER_OP)).toEqual({
			kind: "failed",
			code: "REQUEST_FAILED",
			message: "Could not reach the orderbook",
			retryable: true,
		})
	})
})

describe("what the orderbook's answer means for the bid", () => {
	it("holds the bid when it accepted it now or already had it", () => {
		expect(orderbookHoldsBid(ACCEPTED)).toBe(true)
		expect(orderbookHoldsBid({ ...ACCEPTED, kind: "unchanged" })).toBe(true)
		expect(orderbookHoldsBid(REJECTED)).toBe(false)
		expect(orderbookHoldsBid(FAILED)).toBe(false)
		expect(orderbookHoldsBid(undefined)).toBe(false)
	})

	it("names the refusal by its code, and nothing for a bid it holds", () => {
		expect(bidRefusal(REJECTED)).toBe("NO_LIMIT_ORDER: no limit order on the book")
		expect(bidRefusal(FAILED)).toBe("TIMEOUT: try again")
		expect(bidRefusal(ACCEPTED)).toBeUndefined()
	})
})
