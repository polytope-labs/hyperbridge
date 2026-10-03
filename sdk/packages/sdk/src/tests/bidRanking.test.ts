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
	} as unknown as Bid
}

const bids = new BidManager({} as never, {} as never)

describe("sortBids", () => {
	it("ranks by rate across legs, not size, drops a quote below the order, and leaves payments as signed", async () => {
		// Rates: full and large 1.10, betterPartial 1.20, legOneOnly 1.20 on leg 1 alone. Ties keep arrival order.
		const full = quote([100n, 200n], [110n, 220n])
		const large = quote([200n, 400n], [220n, 440n])
		const betterPartial = quote([50n, 100n], [60n, 120n])
		const legOneOnly = quote([0n, 200n], [0n, 240n])
		// Leg 0 at 0.90 is below the order's 1.00, however good leg 1 is.
		const belowOrder = quote([100n, 100n], [90n, 200n])
		expect(await bids.sortBids(order, [full, belowOrder, large, legOneOnly, betterPartial])).toEqual([
			legOneOnly,
			betterPartial,
			full,
			large,
		])
		expect(betterPartial.outputs.map((asset) => asset.amount)).toEqual([60n, 120n])
	})

	it("ranks by rate, best first, whatever order the bids arrived in and whatever their size", async () => {
		const oneLeg = { inputs: [{ token, amount: 300n }], output: { assets: [{ token, amount: 300n }] } } as Order
		// Arrive worst first. The largest bid has the worst rate, so size alone would pick it.
		const worst = quote([200n], [200n])
		const best = quote([100n], [130n])
		const middle = quote([100n], [115n])
		expect(await bids.sortBids(oneLeg, [worst, best, middle])).toEqual([best, middle, worst])
	})

	it("drops a quote that names no leg or quotes an input without an output", async () => {
		const empty = quote([0n, 0n], [0n, 0n])
		const unpaid = quote([100n, 0n], [0n, 0n])
		const good = quote([100n, 200n], [100n, 200n])
		expect(await bids.sortBids(order, [empty, unpaid, good])).toEqual([good])
	})
})

describe("prepareSubmitBid", () => {
	it("refuses malformed fill calldata before asking the solver to sign", async () => {
		const signTypedData = vi.fn()
		const submitter = new BidManager(
			{} as never,
			{
				decodeERC7821Execute: () => [{ data: "0x68ddf058" }],
			} as never,
		)
		await expect(
			submitter.prepareSubmitBid({
				order,
				callData: "0x",
				fillOptions: { inputs: [] },
				solverSigner: { signTypedData },
			} as never),
		).rejects.toThrow(/quote|malformed/i)
		expect(signTypedData).not.toHaveBeenCalled()
	})
})
