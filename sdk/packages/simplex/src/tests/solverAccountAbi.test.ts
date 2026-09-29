import {
	concatHex,
	decodeErrorResult,
	decodeFunctionData,
	encodeAbiParameters,
	encodeFunctionData,
	getAbiItem,
	parseAbiParameters,
	toFunctionSelector,
} from "viem"
import { expect, it } from "vitest"
import { SOLVER_ACCOUNT_ABI as ABI } from "../config/abis/SolverAccount"

// Selectors taken from the compiled contract.
const SETTLE_BUDGET = "0x391af0f6"
const SPENT = "0xae20bed3"
const BUDGET_EXCEEDED = "0xbfa7ba44"
const ACCOUNT_UNAUTHORIZED = "0x7cf8632b"

const budgetId = "0x1111111111111111111111111111111111111111111111111111111111111111"
const token = "0x2222222222222222222222222222222222222222"
const sender = "0x3333333333333333333333333333333333333333"

it("matches the contract selector for settleBudget", () => {
	expect(toFunctionSelector("settleBudget(bytes32,uint256,address,uint256,uint256)")).toBe(SETTLE_BUDGET)
	expect(toFunctionSelector(getAbiItem({ abi: ABI, name: "settleBudget" }))).toBe(SETTLE_BUDGET)
})

it("matches the contract selector for spent", () => {
	expect(toFunctionSelector("spent(bytes32)")).toBe(SPENT)
	expect(toFunctionSelector(getAbiItem({ abi: ABI, name: "spent" }))).toBe(SPENT)
})

it("round trips settleBudget calldata with the arguments in order", () => {
	const args = [budgetId, 1_000n, token, 400n, 7n] as const

	const data = encodeFunctionData({ abi: ABI, functionName: "settleBudget", args })
	const decoded = decodeFunctionData({ abi: ABI, data })

	expect(data.startsWith(SETTLE_BUDGET)).toBe(true)
	expect(decoded.functionName).toBe("settleBudget")
	expect(decoded.args).toEqual(args)
})

it("decodes a BudgetExceeded revert", () => {
	expect(toFunctionSelector("BudgetExceeded(bytes32,uint256,uint256)")).toBe(BUDGET_EXCEEDED)

	const decoded = decodeErrorResult({
		abi: ABI,
		data: concatHex([
			BUDGET_EXCEEDED,
			encodeAbiParameters(parseAbiParameters("bytes32, uint256, uint256"), [budgetId, 1_200n, 1_000n]),
		]),
	})

	expect(decoded.errorName).toBe("BudgetExceeded")
	expect(decoded.args).toEqual([budgetId, 1_200n, 1_000n])
})

it("decodes an AccountUnauthorized revert", () => {
	expect(toFunctionSelector("AccountUnauthorized(address)")).toBe(ACCOUNT_UNAUTHORIZED)

	const decoded = decodeErrorResult({
		abi: ABI,
		data: concatHex([ACCOUNT_UNAUTHORIZED, encodeAbiParameters(parseAbiParameters("address"), [sender])]),
	})

	expect(decoded.errorName).toBe("AccountUnauthorized")
	expect(decoded.args).toEqual([sender])
})
