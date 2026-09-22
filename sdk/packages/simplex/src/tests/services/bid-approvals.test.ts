import { describe, expect, it } from "vitest"
import { bytes20ToBytes32, type FillOptions, type HexString, type Order } from "@hyperbridge/sdk"
import { decodeAbiParameters, decodeFunctionData, erc20Abi, parseAbi } from "viem"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"

/**
 * The approvals a bid signs ahead of its `fillOrder`.
 *
 * One solver can hold several bids on one order, at different levels of its book, and
 * they execute one after another. An approval skipped because the allowance covered
 * the bid when it was signed is spent by a sibling that fills first, and the bid then
 * reverts with `ERC20InsufficientAllowance`. So every bid sets its own allowance.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const GATEWAY = "0x6CF42FA9BecbC5b6a26884964956b113530f7cFA" as HexString
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const CNGN = "0x2222222222222222222222222222222222222222" as HexString
const EXECUTE = parseAbi(["function execute(bytes32 mode, bytes executionData)"])

function service(): ContractInteractionService {
	const contract = new ContractInteractionService(
		// biome-ignore lint/suspicious/noExplicitAny: no chain is read on this path
		{} as any,
		// biome-ignore lint/suspicious/noExplicitAny: only the gateway address is read
		{ loggers: undefined, getIntentGatewayAddress: () => GATEWAY } as any,
		privateKeySigner(SOLVER_KEY),
	)
	return contract
}

const order: Order = {
	id: "0xorder",
	user: bytes20ToBytes32(USDC),
	source: "EVM-97",
	destination: "EVM-97",
	deadline: 1000n,
	nonce: 0n,
	fees: 0n,
	session: "0x0000000000000000000000000000000000000000",
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: bytes20ToBytes32(USDC), amount: 50n }],
	output: { beneficiary: bytes20ToBytes32(USDC), assets: [{ token: bytes20ToBytes32(CNGN), amount: 78_000n }], call: "0x" },
}

const outputs = [{ token: bytes20ToBytes32(CNGN), amount: 39_750n }]
const options: FillOptions = {
	relayerFee: 0n,
	nativeDispatchFee: 0n,
	validUntil: 0n,
	outputs,
	inputs: [{ token: bytes20ToBytes32(USDC), amount: 25n }],
}

/** The calls in the ERC-7821 batch, with each approval decoded to its spender and amount. */
async function calls(contract: ContractInteractionService) {
	const callData = await contract.buildApprovalAndFillCalldata(order, outputs, options, 0n)
	const { args } = decodeFunctionData({ abi: EXECUTE, data: callData })
	const [batch] = decodeAbiParameters(
		[{ type: "tuple[]", components: [{ name: "target", type: "address" }, { name: "value", type: "uint256" }, { name: "data", type: "bytes" }] }],
		args[1],
	)
	return batch.map((call) => {
		if (call.target.toLowerCase() === GATEWAY.toLowerCase()) return { fill: true }
		const approval = decodeFunctionData({ abi: erc20Abi, data: call.data })
		return { token: call.target.toLowerCase(), fn: approval.functionName, args: approval.args }
	})
}

describe("the approvals a bid signs", () => {
	it("resets and sets the allowance to what this bid pays, before the fill, whatever the allowance was", async () => {
		// No allowance is read at all: what it was when the bid was signed says nothing
		// about what it will be once a sibling bid has filled.
		expect(await calls(service())).toEqual([
			{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
			{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 39_750n] },
			{ fill: true },
		])
	})
})
