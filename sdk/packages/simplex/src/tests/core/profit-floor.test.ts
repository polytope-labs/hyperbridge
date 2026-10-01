import { describe, expect, it, vi } from "vitest"
import type { HexString, Order } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import type { FillerStrategy } from "@/strategies/base"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * The filler refuses an order no strategy scores above zero. An order at or above
 * `minOrderSizeUsd` whose fees fall short of its gas scores the shortfall, and the
 * strategy let it through without the fee check, so the floor does not apply to it.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString

function build(profitability: number, feeCheckWaived: boolean) {
	const strategy = {
		name: "stub",
		calculateProfitability: vi.fn(async () => profitability),
	} as unknown as FillerStrategy

	const filler = new IntentFiller(
		[{ chainId: 8453 } as never],
		[strategy],
		{ maxConcurrentOrders: 1 } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
		} as never,
		{ getPublicClient: () => ({ getBlockNumber: async () => 100n }) } as never,
		{ cacheService: { isPartialFill: () => false, isFeeCheckWaived: () => feeCheckWaived } } as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([8453]) },
	)
	const order = { id: "0xorder", source: "EVM-137", destination: "EVM-8453", deadline: 1_000n } as Order
	// biome-ignore lint/suspicious/noExplicitAny: the evaluation step is what is under test
	const evaluate = () => (filler as any).evaluateOrder(order, new Map([[strategy, true]]))
	return { strategy, evaluate }
}

describe("the profit floor", () => {
	it("refuses an order that scores a loss", async () => {
		const { evaluate } = build(-1, false)

		expect(await evaluate()).toBeNull()
	})

	it("lets through an order whose fee check was waived, at the loss it scored", async () => {
		const { strategy, evaluate } = build(-1, true)

		expect(await evaluate()).toEqual({ strategy, profitability: -1 })
	})
})
