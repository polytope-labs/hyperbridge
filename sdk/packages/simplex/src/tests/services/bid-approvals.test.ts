import { describe, expect, it } from "vitest"
import { bytes20ToBytes32, type FillOptions, type HexString, type Order, type PackedUserOperation } from "@hyperbridge/sdk"
import { decodeAbiParameters, decodeFunctionData, erc20Abi, parseAbi } from "viem"
import { budgetIdFor, type LimitOrderBudget } from "@/orderbook/amounts"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"

/**
 * The approvals a bid signs ahead of its `fillOrder`.
 *
 * One solver can hold several bids on one order, at different levels of its book, and
 * they execute one after another. An approval skipped because the allowance covered
 * the bid when it was signed is spent by a sibling that fills first, and the bid then
 * reverts with `ERC20InsufficientAllowance`. So every bid sets its own allowance.
 *
 * The gateway is approved for exactly what the fill pays, the outputs plus the dispatch
 * fee in the fee token. The fill's gas is paid through the paymaster, and the allowance
 * goes back to zero at the end of the batch.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const SOLVER = privateKeySigner(SOLVER_KEY).address as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
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

/** A service that prepares bids over a chain that answers every read with zero, its fee token USDC at `feeDecimals`. */
function bidService(feeDecimals: number): ContractInteractionService {
	const client = {
		chain: { blockTime: 2000 },
		getBlockNumber: async () => 100n,
		readContract: async () => 0n,
	}
	const contract = new ContractInteractionService(
		// biome-ignore lint/suspicious/noExplicitAny: only the client reads on this path are answered
		{ getPublicClient: () => client, getWalletClient: () => ({}) } as any,
		// biome-ignore lint/suspicious/noExplicitAny: only the settings read on this path are answered
		{
			loggers: undefined,
			getIntentGatewayAddress: () => GATEWAY,
			getSimplexPaymasterAddress: () => undefined,
			getBidValiditySeconds: () => 60,
		} as any,
		privateKeySigner(SOLVER_KEY),
	)
	contract.cacheService.setFeeTokenWithDecimals("EVM-97", USDC, feeDecimals)
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
	const callData = await contract.buildApprovalAndFillCalldata(order, outputs, options)
	return decodeBatch(callData)
}

/** The raw calls in the ERC-7821 batch. */
function rawBatch(callData: HexString) {
	const { args } = decodeFunctionData({ abi: EXECUTE, data: callData })
	const [batch] = decodeAbiParameters(
		[{ type: "tuple[]", components: [{ name: "target", type: "address" }, { name: "value", type: "uint256" }, { name: "data", type: "bytes" }] }],
		args[1],
	)
	return batch
}

function decodeBatch(callData: HexString) {
	return rawBatch(callData).map((call) => {
		if (call.target.toLowerCase() === GATEWAY.toLowerCase()) return { fill: true }
		if (call.target.toLowerCase() === SOLVER.toLowerCase()) return { debit: true }
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
			{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
		])
	})

	it("approves a same-chain fill's outputs alone, and zeroes them at the end", async () => {
		// Nothing is dispatched, so the gateway never pulls the fee token.
		const contract = bidService(6)
		const batch = decodeBatch(
			await contract.buildApprovalAndFillCalldata(order, outputs, { ...options, relayerFee: 120n }),
		)

		expect(batch).toEqual([
			{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
			{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 39_750n] },
			{ fill: true },
			{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
		])
	})

	describe("with several legs", () => {
		const ZERO = "0x0000000000000000000000000000000000000000" as HexString
		const legsOrder = (assets: { token: HexString; amount: bigint }[]): Order => ({
			...order,
			inputs: assets.map(() => ({ token: bytes20ToBytes32(USDC), amount: 50n })),
			output: { ...order.output, assets: assets.map((a) => ({ token: bytes20ToBytes32(a.token), amount: a.amount })) },
		})
		const quote = (assets: { token: HexString; amount: bigint }[]) => ({
			...options,
			outputs: assets.map((a) => ({ token: bytes20ToBytes32(a.token), amount: a.amount })),
			inputs: assets.map(() => ({ token: bytes20ToBytes32(USDC), amount: 25n })),
		})

		it("gives a native output no approval and no reset, and sends its amount with the fill", async () => {
			const assets = [
				{ token: ZERO, amount: 1_000n },
				{ token: CNGN, amount: 39_750n },
			]
			const fillOptions = quote(assets)
			const callData = await service().buildApprovalAndFillCalldata(legsOrder(assets), fillOptions.outputs, fillOptions)
			const batch = rawBatch(callData)

			expect(decodeBatch(callData)).toEqual([
				{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
				{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 39_750n] },
				{ fill: true },
				{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
			])
			expect(batch.map((call) => call.value)).toEqual([0n, 0n, 1_000n, 0n])
			expect(batch.some((call) => call.target.toLowerCase() === ZERO)).toBe(false)
		})

		it("sums two legs paying one ERC20 into one approval and one reset", async () => {
			const assets = [
				{ token: CNGN, amount: 20_000n },
				{ token: CNGN, amount: 19_750n },
			]
			const fillOptions = quote(assets)
			const callData = await service().buildApprovalAndFillCalldata(legsOrder(assets), fillOptions.outputs, fillOptions)

			expect(decodeBatch(callData)).toEqual([
				{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
				{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 39_750n] },
				{ fill: true },
				{ token: CNGN.toLowerCase(), fn: "approve", args: [GATEWAY, 0n] },
			])
		})
	})

	describe("on a cross-chain fill", () => {
		const crossChain: Order = { ...order, id: "0xcross", source: "EVM-84532" }
		/** The order, asking for `token` out. */
		const asking = (token: HexString): Order => ({
			...crossChain,
			output: { ...order.output, assets: [{ token: bytes20ToBytes32(token), amount: 78_000n }] },
		})
		const DISPATCH_FEE = 36_480n
		// Gas priced in the source chain's 18-decimal fee token: 0.010901 USDC.
		const GAS_IN_SOURCE_FEE_TOKEN = 10_901_000_000_000_000n
		const budget: LimitOrderBudget = { budgetId: budgetIdFor("limit-order-1"), cap: 1_000_000n, token: USDC }

		/** What `prepareBidUserOp` signs for the order, from a gas estimate priced in the source fee token. */
		async function signedBatch(paid: { token: HexString; amount: bigint }[], limitOrder?: LimitOrderBudget) {
			const contract = bidService(6)
			let signed: { callData: HexString } | undefined
			const helper = {
				estimateBidPreVerificationGas: async () => 50_000n,
				prepareSubmitBid: async (params: { callData: HexString; nonce: bigint }): Promise<PackedUserOperation> => {
					signed = params
					return {
						sender: SOLVER,
						nonce: params.nonce,
						initCode: "0x",
						callData: params.callData,
						accountGasLimits: `0x${"00".repeat(32)}`,
						preVerificationGas: 0n,
						gasFees: `0x${"00".repeat(32)}`,
						paymasterAndData: "0x",
						signature: "0x",
					}
				},
			}
			// biome-ignore lint/suspicious/noExplicitAny: stands in for the gateway helper that would sign
			;(contract as any).getIntentGateway = async () => helper

			const asked = asking(paid[0].token)
			const id = asked.id as string
			const filled = paid.map(({ token, amount }) => ({ token: bytes20ToBytes32(token), amount }))
			contract.cacheService.setGasEstimate(
				id,
				GAS_IN_SOURCE_FEE_TOKEN,
				0n,
				DISPATCH_FEE,
				1_000_000n,
				100_000n,
				50_000n,
				1n,
				1n,
				1n,
			)
			contract.cacheService.setFillerOutputs(id, filled, [{ token: bytes20ToBytes32(USDC), amount: 25n }], limitOrder)
			await contract.prepareBidUserOp(asked, ENTRY_POINT, SOLVER)
			return decodeBatch((signed as { callData: HexString }).callData)
		}

		const approve = (token: HexString, amount: bigint) => ({
			token: token.toLowerCase(),
			fn: "approve",
			args: [GATEWAY, amount],
		})

		it("approves the outputs and the dispatch fee, with no gas in it", async () => {
			// The destination fee token has 6 decimals and the source's 18. Gas read in the
			// source's decimals once went into this approval as billions of USDC.
			expect(await signedBatch([{ token: USDC, amount: 39_750n }])).toEqual([
				approve(USDC, 0n),
				approve(USDC, 39_750n + DISPATCH_FEE),
				{ fill: true },
				approve(USDC, 0n),
			])
		})

		it("approves the dispatch fee alone in a fee token that is no output", async () => {
			expect(await signedBatch([{ token: CNGN, amount: 39_750n }])).toEqual([
				approve(CNGN, 0n),
				approve(CNGN, 39_750n),
				approve(USDC, 0n),
				approve(USDC, DISPATCH_FEE),
				{ fill: true },
				approve(CNGN, 0n),
				approve(USDC, 0n),
			])
		})

		it("zero-resets the other tokens after the budget call, which clears the budget token itself", async () => {
			const batch = await signedBatch([{ token: CNGN, amount: 39_750n }], { ...budget, token: CNGN })
			const fill = batch.findIndex((call) => "fill" in call)
			const debit = batch.findIndex((call) => "debit" in call)

			expect(fill).toBeGreaterThan(-1)
			expect(debit).toBeGreaterThan(fill)
			// `debitOrder` does `forceApprove(gateway, 0)` on the budget token (CNGN).
			expect(batch.slice(debit + 1)).toEqual([approve(USDC, 0n)])
		})

		it("zero-resets every token when no budget is settled", async () => {
			const batch = await signedBatch([{ token: CNGN, amount: 39_750n }])

			expect(batch.slice(-2)).toEqual([approve(CNGN, 0n), approve(USDC, 0n)])
		})
	})
})
