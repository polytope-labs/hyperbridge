import { afterEach, describe, expect, it, vi } from "vitest"
import { bytes20ToBytes32, type HexString, type Order, type PackedUserOperation } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import type { BidPlan } from "@/services/CacheService"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { limitOrderStore } from "../helpers/limit-orders"

/**
 * The bids on one order go out in turn, and every one is signed with the gas estimate
 * the order's evaluation cached. Hyperbridge can hold each submission in its pool for
 * longer than that cache lives, and the order is never evaluated again, so the estimate
 * has to outlast the loop or the later bids are never sent.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const SOLVER = privateKeySigner(SOLVER_KEY).address as HexString
const GATEWAY = "0x6CF42FA9BecbC5b6a26884964956b113530f7cFA" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const CNGN = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd" as HexString
const ZERO = "0x0000000000000000000000000000000000000000" as HexString
const CHAIN = "EVM-1"
const LIMIT_ORDERS = ["limit-a", "limit-b", "limit-c"]
const PAYOUTS = [1000n, 900n, 800n].map((whole) => whole * 10n ** 18n)

const ORDER: Order = {
	id: "0xorder",
	user: bytes20ToBytes32(USDC),
	source: CHAIN,
	destination: CHAIN,
	deadline: 1000n,
	nonce: 0n,
	fees: 0n,
	session: ZERO,
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: bytes20ToBytes32(USDC), amount: 50n }],
	output: {
		beneficiary: bytes20ToBytes32(USDC),
		assets: [{ token: bytes20ToBytes32(CNGN), amount: 3000n * 10n ** 18n }],
		call: "0x",
	},
}

function plan(limitOrderId: string, payout: bigint): BidPlan {
	return {
		limitOrderId,
		leg: 0,
		payout,
		fillerOutputs: [{ token: bytes20ToBytes32(CNGN), amount: payout }],
		fillerInputs: [{ token: bytes20ToBytes32(USDC), amount: 10n }],
		fundingCalls: [],
		partialFill: false,
		profit: 1,
	}
}

/**
 * A filler over the real bid builder, whose submissions each take `clock.submissionMs`
 * of a clock the test owns. The order's evaluation is cached at the clock's start.
 */
async function build() {
	const clock = { now: Date.now(), submissionMs: 0 }
	vi.spyOn(Date, "now").mockImplementation(() => clock.now)

	const client = {
		chain: { blockTime: 2000 },
		getBlockNumber: async () => 100n,
		getBlock: async () => ({ number: 100n, timestamp: BigInt(Math.floor(clock.now / 1000)) }),
		readContract: async () => 0n,
	}
	const contract = new ContractInteractionService(
		{ getPublicClient: () => client, getWalletClient: () => ({}) } as never,
		{
			loggers: undefined,
			getIntentGatewayAddress: () => GATEWAY,
			getSimplexPaymasterAddress: () => undefined,
			getBidValiditySeconds: () => 60,
		} as never,
		privateKeySigner(SOLVER_KEY),
	)
	const signed: bigint[] = []
	const helper = {
		estimateBidPreVerificationGas: async () => 50_000n,
		prepareSubmitBid: async (params: any): Promise<PackedUserOperation> => {
			signed.push(params.fillOptions.outputs[0].amount)
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

	// What the evaluation leaves behind: the shared gas estimate and one plan per limit order.
	const cache = contract.cacheService
	cache.setGasEstimate(ORDER.id!, 500n, 0n, 120n, 1_000_000n, 100_000n, 50_000n, 1n, 1n, 1n)
	cache.setBidPlans(
		ORDER.id!,
		LIMIT_ORDERS.map((id, index) => plan(id, PAYOUTS[index])),
	)

	const strategy = {
		name: "test",
		canFill: async () => true,
		calculateProfitability: async () => 1,
		executeOrder: async (order: Order) => {
			const { commitment, bid } = await contract.prepareBidUserOp(order, ENTRY_POINT, SOLVER)
			clock.now += clock.submissionMs
			return { success: true, commitment, txHash: "0xtx", bid }
		},
		getOrderUsdValue: async () => ({ inputUsd: { toNumber: () => 1 } }),
	}
	const limitOrders = await limitOrderStore(
		LIMIT_ORDERS.map((id) => ({
			id,
			base: "USDC",
			quote: "CNGN",
			side: "BID" as const,
			fillChain: CHAIN,
			price: "1500",
			size: "3000",
		})),
	)
	const filler = new IntentFiller(
		[{ chainId: 1 } as never],
		[strategy as never],
		{ maxConcurrentOrders: 1 } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getIntentGatewayAddress: () => GATEWAY,
			getRpcUrls: () => ["https://rpc.example"],
		} as never,
		{} as never,
		contract,
		{ address: SOLVER } as never,
		{ orders: stubOrderScanner([1]) },
		undefined,
		new MemoryDataStore().bids,
		limitOrders,
	)
	const executed: { success: boolean }[] = []
	filler.monitor.on("orderExecuted", (event: { success: boolean }) => executed.push(event))
	const execute = async () => {
		await (filler as any).executeOrder(ORDER, (filler as any).strategies[0], false, { toNumber: () => 1 }, 1)
		await (filler as any).chainQueues.get(1)?.onIdle()
	}
	const expiryMs: number = (cache as any).CACHE_EXPIRY_MS
	return { clock, cache, signed, executed, execute, expiryMs }
}

describe("the bids on one order", () => {
	afterEach(() => vi.restoreAllMocks())

	it("all go out when each submission outlasts the evaluation cache", async () => {
		const { clock, cache, signed, executed, execute, expiryMs } = await build()
		clock.submissionMs = expiryMs + 1_000

		await execute()

		expect(signed).toEqual(PAYOUTS)
		expect(executed).toEqual(PAYOUTS.map(() => expect.objectContaining({ success: true })))
		// The evaluation is kept only for the loop: it expires as soon as the last bid is out.
		expect(cache.getGasEstimate(ORDER.id!)).toBeNull()
		expect(cache.getBidPlans(ORDER.id!)).toEqual([])
	})

	it("are not sent on an evaluation that expired before the first one started", async () => {
		const { clock, signed, executed, execute, expiryMs } = await build()
		// Waited in the chain's queue past the cache's expiry: the loop does not revive it.
		clock.now += expiryMs + 1_000

		await execute()

		expect(signed).toEqual([])
		expect(executed).toEqual([])
	})
})
