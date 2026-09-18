import { describe, expect, it, vi } from "vitest"
import { BidManager } from "@/protocols/intents/BidManager"
import type { Bid, HexString, Order } from "@/types"

const token = `0x${"00".repeat(12)}${"11".repeat(20)}` as HexString
const order = {
	inputs: [
		{ token, amount: 100n },
		{ token, amount: 200n },
	],
	output: {
		assets: [
			{ token, amount: 100n },
			{ token, amount: 200n },
		],
	},
} as Order

function quote(takes: bigint[], payments: bigint[]): Bid {
	return {
		inputs: takes.map((amount) => ({ token, amount })),
		outputs: payments.map((amount) => ({ token, amount })),
		simulate: vi.fn(async () => undefined),
		execute: vi.fn(async () => ({ status: "FILLED" })),
	} as unknown as Bid
}

function manager() {
	return new BidManager(
		{
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: {},
			dest: {
				config: { stateMachineId: "EVM-1" },
				configService: {
					getUsdcAsset: () => `0x${"11".repeat(20)}`,
					getUsdtAsset: () => `0x${"22".repeat(20)}`,
					getUsdcDecimals: () => 6,
					getUsdtDecimals: () => 6,
					getWrappedNativeAssetWithDecimals: () => ({ asset: token, decimals: 18 }),
				},
			},
		} as never,
		{} as never,
	)
}

describe("mandatory multi-leg quotes", () => {
	it("ranks price independently of quoted size and keeps signed payments unchanged", async () => {
		const full = quote([100n, 200n], [110n, 220n])
		const large = quote([200n, 400n], [220n, 440n])
		const betterPartial = quote([50n, 100n], [60n, 120n])
		expect(await manager().sortBids(order, [full, large, betterPartial])).toEqual([betterPartial, full, large])
		expect(betterPartial.outputs.map((asset) => asset.amount)).toEqual([60n, 120n])
	})

	it("executes a full-escrow quote with repeated-token legs", async () => {
		const full = quote([100n, 200n], [110n, 220n])
		expect(await manager().selectAndExecuteBest(order, [full])).toEqual({ status: "FILLED" })
	})
	it("refuses malformed v3 calldata before asking the solver to sign", async () => {
		const signTypedData = vi.fn()
		const bids = new BidManager(
			{} as never,
			{
				decodeERC7821Execute: () => [{ data: "0x68ddf058" }],
			} as never,
		)
		await expect(
			bids.prepareSubmitBid({
				order,
				callData: "0x",
				fillOptions: { inputs: [] },
				solverSigner: { signTypedData },
			} as never),
		).rejects.toThrow(/quote|malformed/i)
		expect(signTypedData).not.toHaveBeenCalled()
	})
	it("ranks a mixed-token partial quote without pricing its skipped leg", async () => {
		const exotic = `0x${"00".repeat(12)}${"33".repeat(20)}` as HexString
		const mixed = {
			...order,
			output: {
				assets: [
					{ token: exotic, amount: 100n },
					{ token, amount: 200n },
				],
			},
		} as Order
		const partial = quote([0n, 100n], [0n, 120n])
		partial.outputs[0].token = exotic
		const bids = manager()
		// The DEX dependency refuses a zero-amount swap, as a real router does.
		const dex = bids as unknown as { quoteTokenToUsdc(token: HexString, amount: bigint): Promise<bigint> }
		vi.spyOn(dex, "quoteTokenToUsdc").mockImplementation(async (_token, amount) => {
			if (amount === 0n) throw new Error("Zero amount swap")
			return amount
		})
		expect(await bids.sortBids(mixed, [partial])).toEqual([partial])
	})
})
