import { describe, expect, it } from "vitest"
import {
	bytes20ToBytes32,
	decodeERC7821ExecuteBatch,
	encodeERC7821ExecuteBatch,
	encodeFillOrder,
	transformOrderForContract,
	type ERC7821Call,
	type FillOptions,
	type HexString,
	type Order,
	type PackedUserOperation,
} from "@hyperbridge/sdk"
import { decodeFunctionData, encodeFunctionData, erc20Abi } from "viem"
import { SOLVER_ACCOUNT_ABI } from "@/config/abis/SolverAccount"
import { budgetIdFor, type LimitOrderBudget } from "@/orderbook/amounts"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"

/**
 * The `settleBudget` call a bid priced by a limit order ends with.
 *
 * The account works out what a fill paid from what the gateway left of the
 * allowance: `used = approved - allowance - fee`. So `approved` has to be exactly
 * what the batch approved in the budget's token, and `fee` exactly what the gateway
 * draws from it that is not payout, or the tally drifts from what was paid.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const SOLVER = privateKeySigner(SOLVER_KEY).address as HexString
const GATEWAY = "0x6CF42FA9BecbC5b6a26884964956b113530f7cFA" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const CNGN = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd" as HexString
const ZERO = "0x0000000000000000000000000000000000000000" as HexString
const SETTLE_BUDGET = "0x391af0f6"

const FILL_CHAIN = "EVM-97"
const OTHER_CHAIN = "EVM-84532"

const PAYOUT = 39_750n
const FILL_GAS = 500n
const RELAYER_FEE = 120n
const BASE_CALL_GAS = 1_000_000n
const FUNDING_GAS_PER_CALL = 400_000n
const SETTLE_BUDGET_GAS = 60_000n

const budget: LimitOrderBudget = { budgetId: budgetIdFor("limit-order-1"), cap: 78_000n, token: CNGN }

function orderFrom(source: string): Order {
	return {
		id: `0xorder-${source}`,
		user: bytes20ToBytes32(USDC),
		source,
		destination: FILL_CHAIN,
		deadline: 1000n,
		nonce: 0n,
		fees: 0n,
		session: ZERO,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: bytes20ToBytes32(USDC), amount: 50n }],
		output: {
			beneficiary: bytes20ToBytes32(USDC),
			assets: [{ token: bytes20ToBytes32(CNGN), amount: 78_000n }],
			call: "0x",
		},
	}
}

const sameChain = orderFrom(FILL_CHAIN)
const crossChain = orderFrom(OTHER_CHAIN)

const outputs = [{ token: bytes20ToBytes32(CNGN), amount: PAYOUT }]
const inputs = [{ token: bytes20ToBytes32(USDC), amount: 25n }]

function optionsWith(relayerFee: bigint): FillOptions {
	return { relayerFee, nativeDispatchFee: 0n, validUntil: 0n, outputs, inputs }
}

function service(feeToken: HexString = USDC): ContractInteractionService {
	const client = {
		chain: { blockTime: 2000 },
		getBlockNumber: async () => 100n,
		readContract: async () => 0n,
	}
	const contract = new ContractInteractionService(
		{ getPublicClient: () => client, getWalletClient: () => ({}) } as any,
		{
			loggers: undefined,
			getIntentGatewayAddress: () => GATEWAY,
			getSimplexPaymasterAddress: () => undefined,
			getBidValiditySeconds: () => 60,
		} as any,
		privateKeySigner(SOLVER_KEY),
	)
	contract.cacheService.setFeeTokenWithDecimals(FILL_CHAIN, feeToken, 18)
	return contract
}

function batchOf(callData: HexString): ERC7821Call[] {
	const batch = decodeERC7821ExecuteBatch(callData)
	if (!batch) throw new Error("not an ERC-7821 batch")
	return batch
}

/** The arguments of the batch's last call, which has to be the account settling the budget. */
function settled(batch: ERC7821Call[]) {
	const last = batch[batch.length - 1]
	expect(last.target.toLowerCase()).toBe(SOLVER.toLowerCase())
	expect(last.value).toBe(0n)
	expect(last.data.slice(0, 10)).toBe(SETTLE_BUDGET)
	const { functionName, args } = decodeFunctionData({ abi: SOLVER_ACCOUNT_ABI, data: last.data })
	if (functionName !== "settleBudget") throw new Error(`the batch ends with ${functionName}`)
	const [budgetId, cap, token, approved, fee] = args
	return { budgetId, cap, token: token.toLowerCase(), approved, fee }
}

/** What the batch approves to the gateway in `token`, from its last approval of it. */
function approvedIn(batch: ERC7821Call[], token: HexString): bigint | undefined {
	const approvals = batch
		.filter((call) => call.target.toLowerCase() === token.toLowerCase())
		.map((call) => decodeFunctionData({ abi: erc20Abi, data: call.data }))
	const last = approvals[approvals.length - 1]
	return last?.functionName === "approve" ? last.args[1] : undefined
}

/**
 * Drives `prepareBidUserOp` over a gateway helper that signs nothing and hands back
 * what it was asked to sign.
 */
async function prepareBid(contract: ContractInteractionService, order: Order, cached?: LimitOrderBudget) {
	let signed: any
	const helper = {
		estimateBidPreVerificationGas: async () => 50_000n,
		prepareSubmitBid: async (params: any): Promise<PackedUserOperation> => {
			signed = params
			return {
				sender: params.solverAccount,
				nonce: params.nonce,
				initCode: "0x",
				callData: params.callData,
				accountGasLimits: `0x${"00".repeat(32)}`,
				preVerificationGas: params.preVerificationGas,
				gasFees: `0x${"00".repeat(32)}`,
				paymasterAndData: params.paymasterAndData,
				signature: "0x",
			}
		},
	}
	;(contract as any).getIntentGateway = async () => helper

	const id = order.id as string
	contract.cacheService.setGasEstimate(id, FILL_GAS, 0n, RELAYER_FEE, BASE_CALL_GAS, 100_000n, 50_000n, 1n, 1n, 1n)
	contract.cacheService.setFillerOutputs(id, outputs, inputs, cached)
	await contract.prepareBidUserOp(order, ENTRY_POINT, SOLVER)
	return signed as { callData: HexString; callGasLimit: bigint; fillOptions: FillOptions }
}

describe("the budget call a bid ends with", () => {
	it("settles a same-chain bid for what it approved, with no fee", async () => {
		const callData = await service().buildApprovalAndFillCalldata(sameChain, outputs, optionsWith(0n), 0n, {
			budget,
			solverAccount: SOLVER,
		})

		expect(settled(batchOf(callData))).toEqual({
			budgetId: budget.budgetId,
			cap: budget.cap,
			token: CNGN,
			approved: PAYOUT,
			fee: 0n,
		})
	})

	it("takes the dispatch fee out when the fee token is the token paid", async () => {
		const callData = await service(CNGN).buildApprovalAndFillCalldata(
			crossChain,
			outputs,
			optionsWith(RELAYER_FEE),
			FILL_GAS + RELAYER_FEE,
			{ budget, solverAccount: SOLVER },
		)
		const batch = batchOf(callData)
		const { approved, fee } = settled(batch)

		expect(approved).toBe(PAYOUT + FILL_GAS + RELAYER_FEE)
		expect(approved).toBe(approvedIn(batch, CNGN))
		expect(fee).toBe(RELAYER_FEE)

		// The gateway draws the payout and the dispatch fee, and the tally comes to the
		// payout alone, whether the fill was whole or clamped.
		for (const paid of [PAYOUT, 30_000n]) {
			const allowanceLeft = approved - paid - RELAYER_FEE
			expect(approved - allowanceLeft - fee).toBe(paid)
		}
	})

	it("takes the dispatch fee out even when the fill names a native dispatch fee", async () => {
		const callData = await service(CNGN).buildApprovalAndFillCalldata(
			crossChain,
			outputs,
			{ ...optionsWith(RELAYER_FEE), nativeDispatchFee: 1n },
			FILL_GAS + RELAYER_FEE,
			{ budget, solverAccount: SOLVER },
		)
		const { approved, fee } = settled(batchOf(callData))

		expect(approved).toBe(PAYOUT + FILL_GAS + RELAYER_FEE)
		expect(fee).toBe(RELAYER_FEE)

		// The batch carries no native value to pay the dispatch with, so the gateway
		// still draws the fee from the allowance.
		const allowanceLeft = approved - PAYOUT - RELAYER_FEE
		expect(approved - allowanceLeft - fee).toBe(PAYOUT)
	})

	it("counts no fee when the fee token is another token", async () => {
		const callData = await service(USDC).buildApprovalAndFillCalldata(
			crossChain,
			outputs,
			optionsWith(RELAYER_FEE),
			FILL_GAS + RELAYER_FEE,
			{ budget, solverAccount: SOLVER },
		)
		const batch = batchOf(callData)

		expect(settled(batch)).toMatchObject({ approved: PAYOUT, fee: 0n })
		expect(approvedIn(batch, CNGN)).toBe(PAYOUT)
		expect(approvedIn(batch, USDC)).toBe(FILL_GAS + RELAYER_FEE)
	})

	it("matches the budget's token whatever case it is written in", async () => {
		const shouted = `0x${CNGN.slice(2).toUpperCase()}` as HexString
		const callData = await service().buildApprovalAndFillCalldata(sameChain, outputs, optionsWith(0n), 0n, {
			budget: { ...budget, token: shouted },
			solverAccount: SOLVER,
		})

		expect(settled(batchOf(callData))).toMatchObject({ token: CNGN, approved: PAYOUT })
	})

	it("stays last, directly after the fill, when funding calls are prepended", async () => {
		const contract = service()
		const funding: ERC7821Call = { target: "0x0000000000000000000000000000000000000001", value: 0n, data: "0x01" }
		contract.cacheService.setFundingPrepends(sameChain.id as string, [funding])

		const callData = await contract.buildApprovalAndFillCalldata(sameChain, outputs, optionsWith(0n), 0n, {
			budget,
			solverAccount: SOLVER,
		})
		const batch = batchOf(callData)

		expect(batch).toHaveLength(5)
		expect(batch[0]).toEqual(funding)
		expect(batch[3].target.toLowerCase()).toBe(GATEWAY.toLowerCase())
		expect(settled(batch).approved).toBe(PAYOUT)
	})
})

describe("a bid no budget applies to", () => {
	const approve = (amount: bigint): ERC7821Call => ({
		target: CNGN,
		value: 0n,
		data: encodeFunctionData({ abi: erc20Abi, functionName: "approve", args: [GATEWAY, amount] }),
	})
	const unbudgeted = encodeERC7821ExecuteBatch([
		approve(0n),
		approve(PAYOUT),
		{
			target: GATEWAY,
			value: 0n,
			data: encodeFillOrder(transformOrderForContract(sameChain) as any, optionsWith(0n)),
		},
	])

	it("signs the approvals and the fill, and nothing else", async () => {
		expect(await service().buildApprovalAndFillCalldata(sameChain, outputs, optionsWith(0n), 0n)).toBe(unbudgeted)
	})

	it("is what a budgeted bid signs ahead of its budget call", async () => {
		const callData = await service().buildApprovalAndFillCalldata(sameChain, outputs, optionsWith(0n), 0n, {
			budget,
			solverAccount: SOLVER,
		})

		expect(batchOf(callData).slice(0, -1)).toEqual(batchOf(unbudgeted))
	})

	it("includes one whose budget is in the native token", async () => {
		const callData = await service().buildApprovalAndFillCalldata(sameChain, outputs, optionsWith(0n), 0n, {
			budget: { ...budget, token: ZERO },
			solverAccount: SOLVER,
		})

		expect(callData).toBe(unbudgeted)
	})
})

describe("preparing a bid", () => {
	it("settles the budget cached with the outputs it signs", async () => {
		const bid = await prepareBid(service(), sameChain, budget)

		expect(settled(batchOf(bid.callData))).toEqual({
			budgetId: budget.budgetId,
			cap: budget.cap,
			token: CNGN,
			approved: PAYOUT,
			fee: 0n,
		})
	})

	it("takes out the dispatch fee it signs into the fill", async () => {
		const bid = await prepareBid(service(CNGN), crossChain, budget)
		const { approved, fee } = settled(batchOf(bid.callData))

		expect(bid.fillOptions.relayerFee).toBe(RELAYER_FEE)
		expect(fee).toBe(RELAYER_FEE)
		expect(approved).toBe(PAYOUT + FILL_GAS + RELAYER_FEE)
	})

	it("signs no budget call when no limit order priced the bid", async () => {
		const contract = service()
		const bid = await prepareBid(contract, sameChain)

		expect(bid.callData).toBe(
			await contract.buildApprovalAndFillCalldata(sameChain, outputs, bid.fillOptions, FILL_GAS + RELAYER_FEE),
		)
		expect(batchOf(bid.callData).map((call) => call.target.toLowerCase())).not.toContain(SOLVER.toLowerCase())
	})

	it("fails rather than sign a bid that approves nothing in its budget's token", async () => {
		await expect(prepareBid(service(), sameChain, { ...budget, token: USDC })).rejects.toThrow(
			`approves nothing in ${USDC}, the token budget ${budget.budgetId} is measured in`,
		)
	})

	it("allows call gas for the budget call only when the bid carries it", async () => {
		const contract = service()
		const funding: ERC7821Call = { target: "0x0000000000000000000000000000000000000001", value: 0n, data: "0x01" }

		expect((await prepareBid(contract, sameChain)).callGasLimit).toBe(BASE_CALL_GAS)
		expect((await prepareBid(contract, sameChain, budget)).callGasLimit).toBe(BASE_CALL_GAS + SETTLE_BUDGET_GAS)
		expect((await prepareBid(contract, sameChain, { ...budget, token: ZERO })).callGasLimit).toBe(BASE_CALL_GAS)

		contract.cacheService.setFundingPrepends(sameChain.id as string, [funding])
		expect((await prepareBid(contract, sameChain, budget)).callGasLimit).toBe(
			BASE_CALL_GAS + FUNDING_GAS_PER_CALL + SETTLE_BUDGET_GAS,
		)
	})
})
