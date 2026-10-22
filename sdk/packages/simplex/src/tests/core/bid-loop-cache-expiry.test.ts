import { afterEach, describe, expect, it, vi } from "vitest"
import { Decimal } from "decimal.js"
import { bytes20ToBytes32, type HexString, type Order, type PackedUserOperation } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { CacheService, type BidPlan } from "@/services/CacheService"
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
/** What a later evaluation of the same order prices its bids at. */
const REPRICED = [950n, 850n, 750n].map((whole) => whole * 10n ** 18n)

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
 * of a clock the test owns. The order's evaluation is cached at the clock's start, and
 * evaluating it again prices it at `reevaluations.payouts`, or passes on it when null.
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

	// What an evaluation leaves behind: the shared gas estimate and one plan per limit order.
	const cache = contract.cacheService
	const evaluate = (payouts: bigint[]) => {
		cache.setGasEstimate(ORDER.id!, 500n, 0n, 120n, 1_000_000n, 100_000n, 50_000n, 1n, 1n, 1n)
		cache.setBidPlans(
			ORDER.id!,
			LIMIT_ORDERS.map((id, index) => plan(id, payouts[index])),
		)
	}
	evaluate(PAYOUTS)

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
	const reevaluations = { count: 0, payouts: REPRICED as bigint[] | null }
	const reevaluate = async () => {
		reevaluations.count++
		if (!reevaluations.payouts) return null
		evaluate(reevaluations.payouts)
		return { strategy, profitability: 1 }
	}
	const execute = async () => {
		await (filler as any).executeOrder(ORDER, strategy, false, { toNumber: () => 1 }, 1, reevaluate)
		await (filler as any).chainQueues.get(1)?.onIdle()
	}
	const expiryMs: number = (cache as any).CACHE_EXPIRY_MS
	return { clock, cache, signed, executed, execute, expiryMs, reevaluations }
}

describe("the bids on one order", () => {
	afterEach(() => vi.restoreAllMocks())

	it("all go out when each submission outlasts the evaluation cache", async () => {
		const { clock, cache, signed, executed, execute, expiryMs, reevaluations } = await build()
		clock.submissionMs = expiryMs + 1_000

		await execute()

		expect(reevaluations.count).toBe(0)
		expect(signed).toEqual(PAYOUTS)
		expect(executed).toEqual(PAYOUTS.map(() => expect.objectContaining({ success: true })))
		// The evaluation is kept only for the loop: it expires as soon as the last bid is out.
		expect(cache.getGasEstimate(ORDER.id!)).toBeNull()
		expect(cache.getBidPlans(ORDER.id!)).toEqual([])
	})

	it("are priced by a fresh evaluation when the first expired before they started", async () => {
		const { clock, signed, executed, execute, expiryMs, reevaluations } = await build()
		// The confirmation wait outlasted the cache, and so does each submission.
		clock.now += expiryMs + 1_000
		clock.submissionMs = expiryMs + 1_000

		await execute()

		expect(reevaluations.count).toBe(1)
		expect(signed).toEqual(REPRICED)
		expect(executed).toEqual(REPRICED.map(() => expect.objectContaining({ success: true })))
	})

	it("are not sent when the fresh evaluation passes on the order", async () => {
		const { clock, signed, executed, execute, expiryMs, reevaluations } = await build()
		reevaluations.payouts = null
		clock.now += expiryMs + 1_000

		await execute()

		// Nor on the expired evaluation: the pin does not revive it.
		expect(reevaluations.count).toBe(1)
		expect(signed).toEqual([])
		expect(executed).toEqual([])
	})
})

describe("an order whose confirmation wait outlasts its evaluation", () => {
	afterEach(() => vi.restoreAllMocks())

	it("is evaluated again, and bid on, once it is confirmed", async () => {
		const clock = { now: Date.now() }
		vi.spyOn(Date, "now").mockImplementation(() => clock.now)
		const cache = new CacheService()
		const expiryMs: number = (cache as any).CACHE_EXPIRY_MS
		const crossChain = { ...ORDER, source: "EVM-137", destination: "EVM-8453" }
		const SCANNED_BLOCK = `0x${"ab".repeat(32)}`

		let evaluations = 0
		const bids: string[] = []
		const strategy = {
			name: "test",
			canFill: async () => true,
			confirmationPolicy: { getConfirmationBlocks: () => 2 },
			calculateProfitability: async (order: Order) => {
				evaluations++
				cache.setGasEstimate(order.id!, 500n, 0n, 120n, 1_000_000n, 100_000n, 50_000n, 1n, 1n, 1n)
				return 1
			},
			executeOrder: async (order: Order) => {
				// What `prepareBidUserOp` refuses to sign without.
				if (!cache.getGasEstimate(order.id!)) throw new Error("No cached gas estimate")
				bids.push(order.id!)
				return { success: true }
			},
		}
		// The placement has one of its two confirmations when first read, and the second
		// arrives only after the evaluation's cache has expired.
		let reads = 0
		const getTransactionInclusion = vi.fn(async () => {
			if (reads++ > 0) clock.now += expiryMs + 1_000
			return { blockHash: SCANNED_BLOCK, blockNumber: 100n, confirmations: reads > 1 ? 2n : 1n }
		})
		const readContract = async () => 5n
		const quorum = {
			size: 1,
			getTransactionInclusion,
			readWithQuorum: async (_label: string, read: (client: unknown) => Promise<unknown>) =>
				read({ readContract }),
		}

		const filler = new IntentFiller(
			[{ chainId: 137 } as never, { chainId: 8453 } as never],
			[strategy as never],
			{ maxConcurrentOrders: 1 } as never,
			{
				loggers: undefined,
				getHyperbridgeWsUrl: () => undefined,
				getSubstratePrivateKey: () => undefined,
				getConfiguredChainIds: () => [137, 8453],
				isUserAllowed: () => true,
				getIntentGatewayAddress: () => GATEWAY,
			} as never,
			{
				getPublicClient: () => ({ readContract, getBlockNumber: async () => 100n }),
				getQuorumClient: () => quorum,
			} as never,
			{
				cacheService: cache,
				getCache: () => ({ getSolverSelection: () => false }),
				getInputUsdValue: async () => new Decimal(1000),
			} as never,
			{ address: SOLVER } as never,
			{ orders: stubOrderScanner([137, 8453]) },
			undefined,
			new MemoryDataStore().bids,
		)

		;(filler as any).handleNewOrder(crossChain, "0xtx", SCANNED_BLOCK)
		await (filler as any).globalQueue.onIdle()
		await (filler as any).chainQueues.get(8453)?.onIdle()

		expect(getTransactionInclusion).toHaveBeenCalledTimes(2)
		expect(evaluations).toBe(2)
		expect(bids).toEqual([crossChain.id])
	})
})
