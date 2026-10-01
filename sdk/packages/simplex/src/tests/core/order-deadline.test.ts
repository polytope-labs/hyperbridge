import { describe, expect, it, vi } from "vitest"
import type { HexString, Order } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import type { FillerStrategy } from "@/strategies/base"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * An order's deadline is a block number on its destination chain, and `fillOrder`
 * reverts `Expired` once that chain is past it. The placer fixes the deadline when the
 * order is built, so an order that takes long enough to land on its source chain
 * arrives already expired. No strategy can fill it, and it is passed on before any
 * strategy prices it.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const HEAD = 52_006_493n

function build(getBlockNumber: () => Promise<bigint>) {
	const strategy = {
		name: "stub",
		calculateProfitability: vi.fn(async () => 1),
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
		{ getPublicClient: () => ({ getBlockNumber }) } as never,
		{ cacheService: { isPartialFill: () => false } } as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([8453]) },
	)

	const skipped: { orderId?: string; reason?: string }[] = []
	filler.monitor.on("orderSkipped", (event) => skipped.push(event))
	return { filler, strategy, skipped }
}

function evaluate(filler: IntentFiller, strategy: FillerStrategy, deadline: bigint) {
	const order = { id: "0xorder", source: "EVM-137", destination: "EVM-8453", deadline } as Order
	// biome-ignore lint/suspicious/noExplicitAny: the evaluation step is what is under test
	return (filler as any).evaluateOrder(order, new Map([[strategy, true]]))
}

describe("an order's deadline on its destination chain", () => {
	it("passes on an order whose deadline is behind the destination head", async () => {
		const { filler, strategy, skipped } = build(async () => HEAD)

		expect(await evaluate(filler, strategy, HEAD - 11n)).toBeNull()
		expect(strategy.calculateProfitability).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: "0xorder", reason: "Order deadline has passed on the destination chain" }])
	})

	it("passes on an order whose deadline is the head: a fill lands in a later block", async () => {
		const { filler, strategy } = build(async () => HEAD)

		expect(await evaluate(filler, strategy, HEAD)).toBeNull()
		expect(strategy.calculateProfitability).not.toHaveBeenCalled()
	})

	it("prices an order whose deadline is ahead of the head", async () => {
		const { filler, strategy, skipped } = build(async () => HEAD)

		expect(await evaluate(filler, strategy, HEAD + 1n)).toEqual({ strategy, profitability: 1 })
		expect(skipped).toEqual([])
	})

	it("prices the order when the destination head cannot be read", async () => {
		const { filler, strategy } = build(async () => {
			throw new Error("rpc down")
		})

		expect(await evaluate(filler, strategy, HEAD - 11n)).toEqual({ strategy, profitability: 1 })
	})
})
