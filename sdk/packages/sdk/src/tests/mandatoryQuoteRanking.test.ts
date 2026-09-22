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
	it("refuses malformed fill calldata before asking the solver to sign", async () => {
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
	it("ranks by rate across legs, drops a quote below the order, and prices nothing", async () => {
		// Rates: 330/300 = 1.10, 180/150 = 1.20 on both legs, 240/200 = 1.20 on leg 1 alone.
		const both = quote([100n, 200n], [110n, 220n])
		const betterBoth = quote([50n, 100n], [60n, 120n])
		const legOneOnly = quote([0n, 200n], [0n, 240n])
		// Leg 0 at 0.90 is below the order's 1.00, however good leg 1 is.
		const belowOrder = quote([100n, 100n], [90n, 200n])
		const bids = manager()
		expect(await bids.sortBids(order, [both, belowOrder, legOneOnly, betterBoth])).toEqual([
			legOneOnly,
			betterBoth,
			both,
		])
	})

	it("drops a quote that names no leg or quotes an input without an output", async () => {
		const empty = quote([0n, 0n], [0n, 0n])
		const unpaid = quote([100n, 0n], [0n, 0n])
		const good = quote([100n, 200n], [100n, 200n])
		expect(await manager().sortBids(order, [empty, unpaid, good])).toEqual([good])
	})
})
