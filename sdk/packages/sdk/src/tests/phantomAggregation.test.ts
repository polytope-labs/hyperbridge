import { encodeFunctionData } from "viem"
import { encodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { extractFillData, weightedMedian, FILL_ORDER_ABI, type HexString } from "@/protocols/intents/phantom-aggregation"

const GATEWAY = "0x2d61624A17f361020679FaA16fbB566C344AaF4B"
// USDC and USDT addresses left-padded to bytes32, as they appear in an order's token fields.
const USDC_BYTES32 = "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" as HexString
const USDT_BYTES32 = "0x000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7" as HexString
const SOLVER_AMOUNT = 1_000_000n

// A phantom order as it arrives in a bid: zero output amount (the solver's real quote lives in the
// FillOptions outputs), distinct source and destination.
function phantomOrder() {
	return {
		user: `0x${"00".repeat(32)}`,
		source: "0x6131", // "a1"
		destination: "0x6232", // "b2"
		deadline: 0n,
		nonce: 7n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: USDC_BYTES32, amount: 5_000_000n }],
		output: {
			beneficiary: `0x${"00".repeat(32)}`,
			assets: [{ token: USDT_BYTES32, amount: 0n }],
			call: "0x",
		},
	}
}

function fillOptions() {
	return {
		relayerFee: 0n,
		nativeDispatchFee: 0n,
		outputs: [{ token: USDT_BYTES32, amount: SOLVER_AMOUNT }],
	}
}

// Encodes a fillOrder call wrapped in an ERC-7821 execute batch, the way a solver's bid arrives.
function bidCalldata(target: string = GATEWAY): HexString {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const fillCalldata = (encodeFunctionData as any)({
		abi: FILL_ORDER_ABI,
		functionName: "fillOrder",
		args: [phantomOrder(), fillOptions()],
	}) as HexString
	return encodeERC7821ExecuteBatch([{ target: target as HexString, value: 0n, data: fillCalldata }])
}

describe("extractFillData", () => {
	it("decodes the order, output token, and solver amount from a bid's ERC-7821 batch", () => {
		const result = extractFillData(bidCalldata(), GATEWAY)

		expect(result).not.toBeNull()
		expect(result!.outputToken.toLowerCase()).toBe(USDT_BYTES32.toLowerCase())
		expect(result!.solverAmount).toBe(SOLVER_AMOUNT)
		// The decoded order still carries the phantom's zero output amount.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		expect((result!.order as any).output.assets[0].amount).toBe(0n)
	})

	it("returns null when no inner call targets the gateway", () => {
		const other = "0x9999999999999999999999999999999999999999"
		expect(extractFillData(bidCalldata(other), GATEWAY)).toBeNull()
	})

	it("returns null for calldata that is not an ERC-7821 batch", () => {
		expect(extractFillData("0xdeadbeef", GATEWAY)).toBeNull()
	})
})

describe("weightedMedian", () => {
	it("equals the single quote when there is only one", () => {
		expect(weightedMedian([{ price: 100n, weight: 5n }])).toBe(100n)
	})

	it("weights quotes by balance — the high-liquidity solver pulls the median to its price", () => {
		const quotes = [
			{ price: 100n, weight: 1n },
			{ price: 200n, weight: 1n },
			{ price: 300n, weight: 100n },
		]
		// Total weight 102; cumulative reaches half (>=51) only at price 300.
		expect(weightedMedian(quotes)).toBe(300n)
	})

	it("reduces to the lower median when all weights are equal", () => {
		const quotes = [
			{ price: 10n, weight: 7n },
			{ price: 20n, weight: 7n },
			{ price: 30n, weight: 7n },
		]
		expect(weightedMedian(quotes)).toBe(20n)
	})

	it("ignores zero-weight quotes so a solver with no liquidity has no influence", () => {
		const quotes = [
			{ price: 1n, weight: 0n },
			{ price: 500n, weight: 0n },
			{ price: 100n, weight: 10n },
		]
		expect(weightedMedian(quotes)).toBe(100n)
	})

	it("falls back to the unweighted median when every weight is zero", () => {
		const quotes = [
			{ price: 30n, weight: 0n },
			{ price: 10n, weight: 0n },
			{ price: 20n, weight: 0n },
		]
		expect(weightedMedian(quotes)).toBe(20n)
	})

	it("returns the smallest price whose cumulative weight reaches half the total", () => {
		const quotes = [
			{ price: 10n, weight: 3n },
			{ price: 20n, weight: 4n },
			{ price: 30n, weight: 3n },
		]
		// Total 10; cumulative: 3 (10), 7 (20) — 7*2>=10 → median is 20.
		expect(weightedMedian(quotes)).toBe(20n)
	})
})
