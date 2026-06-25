import { encodeFunctionData } from "viem"
import { encodeERC7821ExecuteBatch } from "@hyperbridge/sdk/intents-helpers"

import {
	buildSimulationOrder,
	erc20AllowanceSlot,
	erc20BalanceSlot,
	extractFillData,
	FILL_ORDER_ABI,
	hasTokenSlotOverride,
	ordersStorageSlot,
	SIM_DEADLINE,
	tokenSlots,
} from "../phantom-simulation.helpers"

type HexString = `0x${string}`

const GATEWAY = "0x2d61624A17f361020679FaA16fbB566C344AaF4B"
const SOLVER = "0x1111111111111111111111111111111111111111"
const SOLVER_PADDED = "0x0000000000000000000000001111111111111111111111111111111111111111"
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

describe("buildSimulationOrder", () => {
	it("points the output at the solver for solverAmount so the fill is not a no-op", () => {
		const order = phantomOrder()
		const modified = buildSimulationOrder(order, SOLVER, SOLVER_AMOUNT) as any

		// Regression guard: a zero output amount makes _fillSameChain skip the transfer entirely.
		expect(modified.output.assets[0].amount).toBe(SOLVER_AMOUNT)
		expect(modified.output.beneficiary.toLowerCase()).toBe(SOLVER_PADDED.toLowerCase())
	})

	it("leaves session as a decoded address string rather than a bigint", () => {
		const order = phantomOrder()
		const modified = buildSimulationOrder(order, SOLVER, SOLVER_AMOUNT) as any

		// Regression guard: assigning 0n here throws in viem's encoder and kills every simulation.
		expect(typeof modified.session).toBe("string")
		expect(modified.session).toBe(order.session)
	})

	it("matches source to destination and sets the simulation deadline", () => {
		const order = phantomOrder()
		const modified = buildSimulationOrder(order, SOLVER, SOLVER_AMOUNT) as any

		expect(modified.source).toBe(order.destination)
		expect(modified.deadline).toBe(SIM_DEADLINE)
	})

	it("preserves the other order fields", () => {
		const order = phantomOrder()
		const modified = buildSimulationOrder(order, SOLVER, SOLVER_AMOUNT) as any

		expect(modified.nonce).toBe(order.nonce)
		expect(modified.user).toBe(order.user)
		expect(modified.inputs).toEqual(order.inputs)
	})

	it("produces an order viem can encode (so the simulation call can be built)", () => {
		const modified = buildSimulationOrder(phantomOrder(), SOLVER, SOLVER_AMOUNT)
		expect(() =>
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			(encodeFunctionData as any)({
				abi: FILL_ORDER_ABI,
				functionName: "fillOrder",
				args: [modified, fillOptions()],
			}),
		).not.toThrow()
	})
})

describe("tokenSlots", () => {
	it("returns the configured override for a known token", () => {
		// USDC (Circle FiatToken) lives at balance slot 9, allowance slot 10.
		expect(tokenSlots("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")).toEqual({
			balanceSlot: 9n,
			allowanceSlot: 10n,
		})
	})

	it("is case-insensitive on the token address", () => {
		expect(tokenSlots("0xA0B86991C6218B36C1D19D4A2E9EB0CE3606EB48")).toEqual({
			balanceSlot: 9n,
			allowanceSlot: 10n,
		})
	})

	it("falls back to the OZ default for an unknown token", () => {
		expect(tokenSlots("0x000000000000000000000000000000000000dead")).toEqual({
			balanceSlot: 0n,
			allowanceSlot: 1n,
		})
	})
})

describe("hasTokenSlotOverride", () => {
	it("is true for a configured token and false for an unknown one", () => {
		expect(hasTokenSlotOverride("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")).toBe(true)
		expect(hasTokenSlotOverride("0xA0B86991C6218B36C1D19D4A2E9EB0CE3606EB48")).toBe(true)
		expect(hasTokenSlotOverride("0x000000000000000000000000000000000000dead")).toBe(false)
	})
})

describe("storage slot helpers", () => {
	it("derive deterministic 32-byte slots that differ between balance and allowance layouts", () => {
		const balance = erc20BalanceSlot(SOLVER as HexString, 9n)
		const allowance = erc20AllowanceSlot(SOLVER as HexString, GATEWAY as HexString, 10n)
		const orders = ordersStorageSlot(`0x${"ab".repeat(32)}`, USDC_BYTES32)

		for (const slot of [balance, allowance, orders]) {
			expect(slot).toMatch(/^0x[0-9a-f]{64}$/)
		}
		expect(balance).not.toBe(allowance)
		// Same inputs always hash to the same slot.
		expect(erc20BalanceSlot(SOLVER as HexString, 9n)).toBe(balance)
	})
})
