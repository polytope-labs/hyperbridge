import { FXFiller, type TradingPair } from "@/strategies/fx"
import { AssetRegistry } from "@/config/asset-registry"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { parseUnits } from "viem"
import type { LimitOrderStore } from "@/data/types"
import { budgetIdFor, toRaw } from "@/orderbook/amounts"
import { limitOrderStore } from "../helpers/limit-orders"

// Pins the fill amount `calculateProfitability` caches — the figure
// `prepareBidUserOp` signs into the bid verbatim — against the matched limit
// order.
//
// The payout is `min(offer, remaining − reserved)` and then whatever the wallet
// can actually cover, so these drive the evaluation with mocked chain access and
// assert the cached output for each of those three outcomes in turn.

const CHAIN = "EVM-97"
const STABLE = "0x1111111111111111111111111111111111111111" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

// 100 in; the order asks for 149,000 EXOTIC (rate 1490, below the limit order's
// 1500), so what the operator offers strictly exceeds the ask.
const INPUT_AMOUNT = parseUnits("100", 18)
const REQUESTED_OUTPUT = parseUnits("149000", 18)
const OFFERED_OUTPUT = parseUnits("150000", 18)

const configService = {
	getUsdcAsset: () => STABLE,
	getUsdtAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
	getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
	getCNgnAsset: () => undefined,
	getMaxOverfillBps: () => 500n,
	getMaxConsecutiveClamps: () => 3,
	// No paymaster configured → paymasterReserveForToken contributes nothing.
	getSimplexPaymasterAddress: () => undefined,
} as any

/**
 * Contract service covering everything `calculateProfitability` touches, with
 * gas priced at zero so the payout under test is the only moving part. Exposes
 * the filler-output and partial-fill caches for assertions.
 */
function makeEvalContractService(
	decimals: Record<string, number> = {},
	/** The account's tally for a limit order, in the token's own units. Nothing spent by default. */
	spent: (budgetId: HexString) => bigint | null = () => 0n,
): any {
	const spentReads: HexString[] = []
	const classifications = new Map<string, unknown>()
	const outputs = new Map<string, TokenInfo[]>()
	const inputs = new Map<string, TokenInfo[]>()
	const partials = new Map<string, boolean>()
	const bidPlans = new Map<string, unknown>()
	return {
		getTokenDecimals: async (token: string) => decimals[token.toLowerCase()] ?? 18,
		getFeeTokenWithDecimals: async () => ({ address: STABLE, decimals: 18 }),
		estimateGasFillPost: async () => ({
			totalCostInSourceFeeToken: 0n,
			relayerFeeInSourceFeeToken: 0n,
			dispatchFee: 0n,
		}),
		// No prior partial fills on-chain: an under-fill stays eligible.
		partialFillsFor: async () => [0n],
		limitOrderSpent: async (_chain: string, budgetId: HexString) => {
			spentReads.push(budgetId)
			return spent(budgetId)
		},
		spentReads,
		cacheService: {
			getPairClassifications: (id: string) => classifications.get(id),
			setPairClassifications: (id: string, pairs: unknown) => classifications.set(id, pairs),
			getFillerOutputs: (id: string) => outputs.get(id),
			setFillerOutputs: (id: string, value: TokenInfo[], takes: TokenInfo[]) => {
				outputs.set(id, value)
				inputs.set(id, takes)
			},
			setMatchedLimitOrder: () => {},
			setBidPlans: (id: string, plans: unknown) => bidPlans.set(id, plans),
			getBidPlans: (id: string) => bidPlans.get(id) ?? [],
			clearBidPlans: (id: string) => bidPlans.delete(id),
			clearPartialFill: (id: string) => partials.delete(id),
			clearFeeCheckWaived: () => {},
			setFeeCheckWaived: () => {},
			setPartialFill: (id: string, value: boolean) => partials.set(id, value),
			getPartialFill: (id: string) => partials.get(id),
			setFundingPrepends: () => {},
			clearFundingPrepends: () => {},
		},
		outputs,
		inputs,
		partials,
		plans: bidPlans,
	}
}

/** Public client stub: balances by lowercase token address, everything else static. */
function makeClientManager(balances: Record<string, bigint>): any {
	const client = {
		chain: { blockTime: 2000 },
		getBlock: async () => ({ number: 100n, timestamp: 1_000_000n }),
		getBalance: async () => 0n,
		readContract: async ({ address }: { address: string }) => balances[address.toLowerCase()] ?? 0n,
	}
	return { getPublicClient: () => client }
}

async function makeFiller(options: {
	contractService: any
	balances: Record<string, bigint>
	/** What the operator is offering to pay out, in whole EXOTIC. Defaults to plenty. */
	offering?: string
	/** A whole book, when one order is not the point of the case. */
	book?: { price: string; size: string; id?: string; side?: "BID" | "ASK" }[]
	/** A store the case keeps hold of, to work its orders down between evaluations. */
	limitOrders?: LimitOrderStore
	/** Collects the strategy's log lines, for the cases that assert why it passed. */
	logged?: { message: string; fields: Record<string, unknown> }[]
}): Promise<FXFiller> {
	const record = (fields: Record<string, unknown>, message: string) => options.logged?.push({ message, fields })
	const logger = { trace: record, debug: record, info: record, warn: record, error: record, fatal: record }
	const registry = new AssetRegistry(configService, { EXOTIC: { [CHAIN]: EXOTIC } })
	const pairs: TradingPair[] = [{ token0: "USDC", token1: "EXOTIC" }]
	const signer = { address: SOLVER } as any
	return new FXFiller(
		signer,
		options.logged ? { ...configService, loggers: { get: () => logger } } : configService,
		makeClientManager(options.balances),
		options.contractService,
		pairs,
		registry,
		{
			limitOrders:
				options.limitOrders ??
				(await limitOrderStore(
					(options.book ?? [{ price: "1500", size: options.offering ?? "1000000" }]).map((order) => ({
						base: "USDC",
						quote: "EXOTIC",
						side: order.side ?? ("BID" as const),
						fillChain: CHAIN,
						price: order.price,
						size: order.size,
						acceptedSources: [CHAIN, "EVM-1"],
						id: order.id,
					})),
				)),
		},
	)
}

/** USDC→EXOTIC, same-chain unless a source is given. Partial-fill eligible: no calldata. */
function makeOrder(id: string, source: string = CHAIN, outputCall: HexString = "0x" as HexString): Order {
	const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT }]
	const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(EXOTIC), amount: REQUESTED_OUTPUT }]
	return {
		id,
		user: bytes20ToBytes32(SOLVER),
		source,
		destination: CHAIN,
		deadline: 0n,
		nonce: 0n,
		// Covers the (zero) execution cost with room to spare, so GATE 1 passes
		// and the evaluation reaches its cached-outputs answer.
		fees: parseUnits("1", 18),
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs,
		output: { beneficiary: bytes20ToBytes32(SOLVER), assets: outputs, call: outputCall },
	} as unknown as Order
}

describe("FXFiller limit order payout", () => {

	it("sends one bid only when the order carries output calldata", async () => {
		// The attached call runs only on a full fill, so the gateway answers anything
		// less with `PartialFillNotAllowed`. A second bid could never add to the
		// first; it would just burn gas reverting once the first one landed.
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			book: [
				{ id: "tight", price: "1500", size: "1000000" },
				{ id: "wide", price: "1600", size: "1000000" },
			],
		})

		await filler.calculateProfitability(makeOrder("payout-calldata", CHAIN, "0xdeadbeef" as HexString))

		const plans = contractService.plans.get("payout-calldata") as { limitOrderId: string }[]
		expect(plans).toHaveLength(1)
		// The one bid it takes is the best offer's.
		expect(plans[0].limitOrderId).toBe("wide")
	})

	it("sends one bid per limit order, each at that order's own rate against the whole input", async () => {
		// Two orders that both clear the ask. Each is its own fill: the gateway clamps
		// whichever lands against what is still outstanding, so neither bid is sized
		// against the other and nothing here adds them up. Summing them would bill one
		// input to both orders at once.
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			book: [
				{ id: "tight", price: "1500", size: "1000000" },
				{ id: "wide", price: "1600", size: "1000000" },
			],
		})

		await filler.calculateProfitability(makeOrder("payout-two-orders"))

		const plans = contractService.plans.get("payout-two-orders") as {
			limitOrderId: string
			fillerOutputs: { amount: bigint }[]
		}[]
		expect(plans).toHaveLength(2)
		// Best offer first: that is the order the bids go out in.
		expect(plans.map((plan) => plan.limitOrderId)).toEqual(["wide", "tight"])
		// Each bids its own offer for the whole input, so the two rank by price.
		expect(plans.map((plan) => plan.fillerOutputs[0].amount)).toEqual([
			parseUnits("160000", 18),
			parseUnits("150000", 18),
		])
	})
	it("bids the limit order's own rate, not the ask", async () => {
		// The gateway credits the swapper the ask and pays the swapper and the protocol
		// whatever the bid offers above it. Bidding the order's own rate is what lets a
		// better-priced operator's bid rank above a worse one.
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
		})

		const profit = await filler.calculateProfitability(makeOrder("payout-full"))

		const cached = contractService.outputs.get("payout-full")
		expect(cached).toHaveLength(1)
		expect(cached![0].token).toBe(bytes20ToBytes32(EXOTIC))
		// 100 in at the order's rate of 1500 offers 150,000, and 149,000 was asked.
		expect(cached![0].amount).toBe(OFFERED_OUTPUT)
		// The ask is met in full, so the take is the whole input.
		expect(contractService.inputs.get("payout-full")).toEqual([
			{ token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT },
		])
		// Meeting the ask in full is a full fill, and the order's fees make it score.
		expect(contractService.partials.get("payout-full")).toBe(false)
		expect(profit).toBeGreaterThan(0)
	})

	it("pays no more than the limit order has left", async () => {
		const contractService = makeEvalContractService()
		// The order has 60,000 left of what it offered, below the rate's 150,000
		// and below the 149,000 asked for.
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			offering: "60000",
		})

		await filler.calculateProfitability(makeOrder("payout-capped"))

		const cached = contractService.outputs.get("payout-capped")
		expect(cached).toHaveLength(1)
		expect(cached![0].amount).toBe(parseUnits("60000", 18))
		// An under-fill takes the input its payout buys at the order's own rate:
		// 60,000 at 1,500 is 40. That is below the most the gateway would accept for
		// it (60/149 of the input, the ask's rate), so the bid keeps its own price.
		expect(contractService.inputs.get("payout-capped")).toEqual([
			{ token: bytes20ToBytes32(STABLE), amount: parseUnits("40", 18) },
		])
		// Short of the ask, so this is an under-fill.
		expect(contractService.partials.get("payout-capped")).toBe(true)
	})

	it("takes a cross-chain under-fill as a partial, as the gateway now allows", async () => {
		// `ExtrinsicIntents._fillCrossChain` keeps cumulative progress per output
		// token, clears `_filled` on an under-fill so another solver can finish the
		// order, and releases escrow proportionally. Refusing these would leave the
		// operator's inventory idle against a swap it is priced to serve.
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			offering: "60000",
		})

		await filler.calculateProfitability(makeOrder("payout-cross", "EVM-1"))

		expect(contractService.outputs.get("payout-cross")![0].amount).toBe(parseUnits("60000", 18))
		expect(contractService.partials.get("payout-cross")).toBe(true)
	})

	it("still fills fully when the wallet holds less than the offer but more than the ask", async () => {
		const contractService = makeEvalContractService()
		// Wallet holds 149,500 — between the ask (149,000) and what the order
		// offered (150,000). The bid pays what the wallet holds against the whole
		// input: a rate between the ask and the order's own, and still a full fill.
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("149500", 18) },
		})

		const profit = await filler.calculateProfitability(makeOrder("payout-balance"))

		const cached = contractService.outputs.get("payout-balance")
		expect(cached).toHaveLength(1)
		expect(cached![0].amount).toBe(parseUnits("149500", 18))
		expect(contractService.inputs.get("payout-balance")).toEqual([
			{ token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT },
		])
		expect(contractService.partials.get("payout-balance")).toBe(false)
		expect(profit).toBeGreaterThan(0)
	})

	describe("a multi-leg order", () => {
		/**
		 * Two legs: 100 STABLE for at least 149,000 EXOTIC, and 149,000 EXOTIC for at least
		 * 90 STABLE. A 1,500 bid serves the first; a 1,600 ask (93.125 for 149,000) the second.
		 */
		function twoLegOrder(id: string, outputCall: HexString = "0x" as HexString): Order {
			const order = makeOrder(id, CHAIN, outputCall)
			return {
				...order,
				inputs: [
					{ token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT },
					{ token: bytes20ToBytes32(EXOTIC), amount: REQUESTED_OUTPUT },
				],
				output: {
					...order.output,
					assets: [
						{ token: bytes20ToBytes32(EXOTIC), amount: REQUESTED_OUTPUT },
						{ token: bytes20ToBytes32(STABLE), amount: parseUnits("90", 18) },
					],
				},
			} as Order
		}

		const plenty = {
			[EXOTIC.toLowerCase()]: parseUnits("1000000", 18),
			[STABLE.toLowerCase()]: parseUnits("1000000", 18),
		}

		type Plan = { leg: number; limitOrderId: string; partialFill: boolean; fillerOutputs: TokenInfo[]; fillerInputs: TokenInfo[] }
		const amounts = (assets: TokenInfo[]) => assets.map((asset) => asset.amount)

		it("bids only on the leg its limit order serves, quoting zero on the others", async () => {
			const contractService = makeEvalContractService()
			const filler = await makeFiller({ contractService, balances: plenty, book: [{ id: "bid", price: "1500", size: "1000000" }] })

			await filler.calculateProfitability(twoLegOrder("multi-one"))

			const plans = contractService.plans.get("multi-one") as Plan[]
			expect(plans).toHaveLength(1)
			expect(plans[0].leg).toBe(0)
			expect(amounts(plans[0].fillerOutputs)).toEqual([OFFERED_OUTPUT, 0n])
			expect(amounts(plans[0].fillerInputs)).toEqual([INPUT_AMOUNT, 0n])
			// The other leg stays open, so this bid is a partial fill of the order.
			expect(plans[0].partialFill).toBe(true)
		})

		it("sends one bid per leg its limit orders serve, each at that order's own rate", async () => {
			const contractService = makeEvalContractService()
			const filler = await makeFiller({
				contractService,
				balances: plenty,
				book: [
					{ id: "bid", price: "1500", size: "1000000" },
					{ id: "ask", price: "1600", size: "1000", side: "ASK" },
				],
			})

			await filler.calculateProfitability(twoLegOrder("multi-both"))

			const plans = contractService.plans.get("multi-both") as Plan[]
			expect(plans.map((plan) => [plan.leg, plan.limitOrderId])).toEqual([
				[0, "bid"],
				[1, "ask"],
			])
			expect(amounts(plans[0].fillerOutputs)).toEqual([OFFERED_OUTPUT, 0n])
			// 149,000 EXOTIC in at 1,600 pays 93.125 STABLE.
			expect(amounts(plans[1].fillerOutputs)).toEqual([0n, parseUnits("93.125", 18)])
			expect(amounts(plans[1].fillerInputs)).toEqual([0n, REQUESTED_OUTPUT])
		})

		it("sizes a later leg from what an earlier leg left on a shared limit order", async () => {
			// Both legs take STABLE for EXOTIC and match the one 1,500 order, which has 200,000
			// left. Leg 0 bids its full 150,000; leg 1 bids the 50,000 left rather than being
			// planned at full size and dropped when its hold finds no room.
			const contractService = makeEvalContractService()
			const filler = await makeFiller({ contractService, balances: plenty, book: [{ id: "bid", price: "1500", size: "200000" }] })
			const order = twoLegOrder("multi-shared")
			order.inputs[1] = { token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT }
			order.output.assets[1] = { token: bytes20ToBytes32(EXOTIC), amount: REQUESTED_OUTPUT }

			await filler.calculateProfitability(order)

			const plans = contractService.plans.get("multi-shared") as Plan[]
			expect(plans.map((plan) => plan.leg)).toEqual([0, 1])
			expect(amounts(plans[0].fillerOutputs)).toEqual([OFFERED_OUTPUT, 0n])
			expect(amounts(plans[1].fillerOutputs)).toEqual([0n, parseUnits("50000", 18)])
			// 50,000 at the order's own rate of 1,500 takes 33.33… STABLE, rounded up.
			expect(plans[1].fillerInputs[1].amount).toBe((parseUnits("50000", 18) * 10n ** 18n + parseUnits("1500", 18) - 1n) / parseUnits("1500", 18))
			expect(plans[1].partialFill).toBe(true)
		})

		it("does not bid on one leg of an order whose output calldata forbids partial fills", async () => {
			const contractService = makeEvalContractService()
			const filler = await makeFiller({ contractService, balances: plenty, book: [{ id: "bid", price: "1500", size: "1000000" }] })

			expect(await filler.calculateProfitability(twoLegOrder("multi-calldata", "0xdeadbeef" as HexString))).toBe(0)
			expect(contractService.plans.get("multi-calldata")).toBeUndefined()
		})
	})
})

describe("FXFiller limit order budget", () => {
	const FIRST = "3f2b8c1e-9d4a-4f6b-8a7c-5e1d2c3b4a59"
	const SECOND = "b7a1d2c4-0e5f-4a3b-9c8d-1f2e3d4c5b6a"
	const SIX = { [EXOTIC.toLowerCase()]: 6 }

	type Plan = { limitOrderId: string; payout: bigint; fillerOutputs: TokenInfo[]; budget?: unknown }

	/** The same swap as `makeOrder`, asking for an output token that does not have 18 decimals. */
	function sixDecimalOrder(id: string, input = INPUT_AMOUNT, requested = parseUnits("149000", 6)): Order {
		const order = makeOrder(id)
		return {
			...order,
			inputs: [{ token: bytes20ToBytes32(STABLE), amount: input }],
			output: { ...order.output, assets: [{ token: bytes20ToBytes32(EXOTIC), amount: requested }] },
		} as Order
	}

	it("caps an 18-decimal payout at the order's size, under the order's own id", async () => {
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			book: [{ id: FIRST, price: "1500", size: "1000000" }],
		})

		await filler.calculateProfitability(makeOrder("budget-18"))

		const plans = contractService.plans.get("budget-18") as Plan[]
		expect(plans).toHaveLength(1)
		expect(plans[0].budget).toEqual({
			budgetId: budgetIdFor(FIRST),
			cap: parseUnits("1000000", 18),
			token: EXOTIC,
		})
	})

	it("states the cap in the output token's own units", async () => {
		const contractService = makeEvalContractService(SIX)
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 6) },
			book: [{ id: FIRST, price: "1500", size: "1000000" }],
		})

		await filler.calculateProfitability(sixDecimalOrder("budget-6"))

		const plans = contractService.plans.get("budget-6") as Plan[]
		expect(plans[0].fillerOutputs[0].amount).toBe(parseUnits("150000", 6))
		expect(plans[0].budget).toEqual({
			budgetId: budgetIdFor(FIRST),
			cap: parseUnits("1000000", 6),
			token: EXOTIC,
		})
	})

	it("gives each bid the budget of the limit order that priced it", async () => {
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			book: [
				{ id: FIRST, price: "1500", size: "400000" },
				{ id: SECOND, price: "1600", size: "900000" },
			],
		})

		await filler.calculateProfitability(makeOrder("budget-two"))

		const plans = contractService.plans.get("budget-two") as Plan[]
		expect(plans.map((plan) => [plan.limitOrderId, plan.budget])).toEqual([
			[SECOND, { budgetId: budgetIdFor(SECOND), cap: parseUnits("900000", 18), token: EXOTIC }],
			[FIRST, { budgetId: budgetIdFor(FIRST), cap: parseUnits("400000", 18), token: EXOTIC }],
		])
	})

	it("keeps the cap at the order's whole size once fills have drawn it down", async () => {
		// The tally the cap is checked against counts from the order's first fill, so a
		// cap that shrank with `remaining` would count what was already paid twice.
		const limitOrders = await limitOrderStore([
			{ id: FIRST, base: "USDC", quote: "EXOTIC", side: "BID", fillChain: CHAIN, price: "1500", size: "200000" },
		])
		await limitOrders.drawDown(FIRST, parseUnits("140000", 18).toString())
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			limitOrders,
		})

		await filler.calculateProfitability(makeOrder("budget-drawn"))

		const plans = contractService.plans.get("budget-drawn") as Plan[]
		expect(plans[0].fillerOutputs[0].amount).toBe(parseUnits("60000", 18))
		expect(plans[0].budget).toEqual({
			budgetId: budgetIdFor(FIRST),
			cap: parseUnits("200000", 18),
			token: EXOTIC,
		})
	})

	it("signs whole 1e18 units of a finer token when the wallet's balance sizes the bid", async () => {
		// The hold is kept at 1e18, so an output finer than that is one the hold cannot
		// express, and the fill would be charged at a rate the draw-down cannot follow.
		const contractService = makeEvalContractService({ [EXOTIC.toLowerCase()]: 20 })
		const filler = await makeFiller({
			contractService,
			// Between the ask and the offer, and not on a whole 1e18 unit.
			balances: { [EXOTIC.toLowerCase()]: parseUnits("149500", 20) + 137n },
			book: [{ id: FIRST, price: "1500", size: "1000000" }],
		})

		await filler.calculateProfitability(sixDecimalOrder("budget-20", INPUT_AMOUNT, parseUnits("149000", 20)))

		const plans = contractService.plans.get("budget-20") as Plan[]
		const signed = plans[0].fillerOutputs[0].amount
		expect(signed).toBe(parseUnits("149500", 20) + 100n)
		expect(signed % 100n).toBe(0n)
		expect(signed).toBe(toRaw(plans[0].payout, 20))
		expect(contractService.outputs.get("budget-20")![0].amount).toBe(signed)
	})

	it("signs the wallet's balance as it stands for a token no finer than 1e18", async () => {
		const contractService = makeEvalContractService(SIX)
		const balance = parseUnits("149500.000001", 6)
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: balance },
			book: [{ id: FIRST, price: "1500", size: "1000000" }],
		})

		await filler.calculateProfitability(sixDecimalOrder("budget-6-balance"))

		const plans = contractService.plans.get("budget-6-balance") as Plan[]
		expect(plans[0].fillerOutputs[0].amount).toBe(balance)
		expect(plans[0].fillerOutputs[0].amount).toBe(toRaw(plans[0].payout, 6))
	})

	it("leaves the part of a size the token cannot express unbid, beneath the cap", async () => {
		// 1000.0000005 of a six-decimal token: the size is finer than the token, so the
		// cap truncates to 1000.000000. Each bid is sized from what is left, truncated
		// the same way. Every fill here pays its bid in full, so this says nothing about
		// a fill the gateway clamps: only that the dust in the size is never bid.
		const limitOrders = await limitOrderStore([
			{
				id: FIRST,
				base: "USDC",
				quote: "EXOTIC",
				side: "BID",
				fillChain: CHAIN,
				price: "1.333333333333333333",
				size: "1000.0000005",
			},
		])
		const contractService = makeEvalContractService(SIX)
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 6) },
			limitOrders,
		})
		const cap = parseUnits("1000", 6)

		let paid = 0n
		let fills = 0
		for (; fills < 50; fills++) {
			const id = `budget-rounding-${fills}`
			// Inputs that never land on a whole unit of the output token.
			const input = parseUnits("77.777777777777777777", 18) + BigInt(fills) * 1_234_567_891_234_567n
			await filler.calculateProfitability(sixDecimalOrder(id, input, parseUnits("100", 6)))

			const plans = contractService.plans.get(id) as Plan[] | undefined
			if (!plans) break
			expect(plans).toHaveLength(1)
			expect(plans[0].budget).toMatchObject({ cap })

			paid += plans[0].fillerOutputs[0].amount
			expect(paid).toBeLessThanOrEqual(cap)
			await limitOrders.drawDown(FIRST, plans[0].payout.toString())
		}

		expect(fills).toBeGreaterThan(1)
		expect(fills).toBeLessThan(50)
		// Worked down to the dust the token cannot express, and not a unit past the cap.
		expect(BigInt((await limitOrders.get(FIRST))!.remaining)).toBe(5n * 10n ** 11n)
		expect(paid).toBe(cap)
	})

	describe("against the account's tally on chain", () => {
		const NOTHING_LEFT = "Skipping a bid: the limit order has nothing left on chain"
		const CAP = parseUnits("1000", 6)
		const plenty = { [EXOTIC.toLowerCase()]: parseUnits("1000000", 6) }

		/** A limit order of 1,000 at 1,500 that the store has worked down to 500. */
		async function halfDrawn() {
			const limitOrders = await limitOrderStore([
				{ id: FIRST, base: "USDC", quote: "EXOTIC", side: "BID", fillChain: CHAIN, price: "1500", size: "1000" },
			])
			await limitOrders.drawDown(FIRST, parseUnits("500", 18).toString())
			return limitOrders
		}

		/** One evaluation of a swap the order's rate offers 1,500 for, with the tally at `spent`. */
		async function evaluate(id: string, spent: bigint | null) {
			const contractService = makeEvalContractService(SIX, () => spent)
			const logged: { message: string; fields: Record<string, unknown> }[] = []
			const filler = await makeFiller({ contractService, balances: plenty, limitOrders: await halfDrawn(), logged })
			const profit = await filler.calculateProfitability(
				sixDecimalOrder(id, parseUnits("1", 18), parseUnits("1490", 6)),
			)
			return { profit, logged, contractService, plans: contractService.plans.get(id) as Plan[] | undefined }
		}

		it("sizes the bid from the store when the tally leaves at least as much room", async () => {
			// Equal to the store, and behind it: signed bids that have not executed are in
			// neither figure, so the chain's room is only ever an upper bound.
			for (const spent of [parseUnits("500", 6), parseUnits("100", 6), 0n]) {
				const { plans } = await evaluate(`room-behind-${spent}`, spent)
				expect(plans).toHaveLength(1)
				expect(plans![0].fillerOutputs[0].amount).toBe(parseUnits("500", 6))
			}
		})

		it("sizes the bid from the chain's room when the tally is ahead of the store", async () => {
			// The store says 500 is left; the account has tallied 700 of the 1,000, so it
			// would refuse anything past 300.
			const { plans } = await evaluate("room-ahead", parseUnits("700", 6))

			expect(plans).toHaveLength(1)
			expect(plans![0].fillerOutputs[0].amount).toBe(parseUnits("300", 6))
			expect(plans![0].payout).toBe(parseUnits("300", 18))
			// The budget the bid carries is the order's, whatever the bid was cut to.
			expect(plans![0].budget).toEqual({ budgetId: budgetIdFor(FIRST), cap: CAP, token: EXOTIC })
		})

		it("builds no bid, and says why, when the chain has no room left", async () => {
			for (const spent of [CAP, CAP + 1n]) {
				const { plans, profit, logged } = await evaluate(`room-none-${spent}`, spent)

				expect(plans).toBeUndefined()
				expect(profit).toBe(0)
				expect(logged.filter((line) => line.message === NOTHING_LEFT).map((line) => line.fields)).toEqual([
					expect.objectContaining({ limitOrder: FIRST, spent: spent.toString(), cap: CAP.toString() }),
				])
			}
		})

		it("sizes the bid from the store when the tally cannot be read", async () => {
			const { plans, logged } = await evaluate("room-unread", null)

			expect(plans).toHaveLength(1)
			expect(plans![0].fillerOutputs[0].amount).toBe(parseUnits("500", 6))
			expect(logged.some((line) => line.message === NOTHING_LEFT)).toBe(false)
		})

		it("reads a limit order's tally once per evaluation, however many legs it prices", async () => {
			// Two legs of 0.12 in, each offered 180 by the one order. The chain has room for
			// 300: the first leg takes its 180 and the second what that leaves.
			const contractService = makeEvalContractService(SIX, () => parseUnits("700", 6))
			const filler = await makeFiller({ contractService, balances: plenty, limitOrders: await halfDrawn() })
			const leg = { token: bytes20ToBytes32(STABLE), amount: parseUnits("0.12", 18) }
			const asked = { token: bytes20ToBytes32(EXOTIC), amount: parseUnits("179", 6) }
			const twoLegs = (id: string) => {
				const order = sixDecimalOrder(id)
				return { ...order, inputs: [leg, leg], output: { ...order.output, assets: [asked, asked] } } as Order
			}

			await filler.calculateProfitability(twoLegs("room-legs"))

			const plans = contractService.plans.get("room-legs") as Plan[]
			expect(plans.map((plan) => plan.fillerOutputs.map((output) => output.amount))).toEqual([
				[parseUnits("180", 6), 0n],
				[0n, parseUnits("120", 6)],
			])
			expect(contractService.spentReads).toEqual([budgetIdFor(FIRST)])

			// Nothing is kept for the next evaluation: it reads the tally afresh.
			await filler.calculateProfitability(twoLegs("room-legs-again"))
			expect(contractService.spentReads).toEqual([budgetIdFor(FIRST), budgetIdFor(FIRST)])
		})

		it("reads each limit order's own tally when several price one order", async () => {
			const spent: Record<string, bigint> = { [budgetIdFor(FIRST)]: parseUnits("400000", 6) }
			const contractService = makeEvalContractService(SIX, (budgetId) => spent[budgetId] ?? 0n)
			const filler = await makeFiller({
				contractService,
				balances: plenty,
				book: [
					{ id: FIRST, price: "1500", size: "500000" },
					{ id: SECOND, price: "1600", size: "500000" },
				],
			})

			await filler.calculateProfitability(sixDecimalOrder("room-two"))

			const plans = contractService.plans.get("room-two") as Plan[]
			// The better offer is untouched; the other is cut to the 100,000 its tally leaves.
			expect(plans.map((plan) => [plan.limitOrderId, plan.fillerOutputs[0].amount])).toEqual([
				[SECOND, parseUnits("160000", 6)],
				[FIRST, parseUnits("100000", 6)],
			])
			expect(contractService.spentReads).toEqual([budgetIdFor(SECOND), budgetIdFor(FIRST)])
		})

		it("reads no tally for an order no limit order prices", async () => {
			// Asks for more than the order's rate offers, so nothing matches it.
			const contractService = makeEvalContractService(SIX)
			const filler = await makeFiller({ contractService, balances: plenty, limitOrders: await halfDrawn() })

			const profit = await filler.calculateProfitability(
				sixDecimalOrder("room-unmatched", parseUnits("1", 18), parseUnits("1501", 6)),
			)

			expect(profit).toBe(0)
			expect(contractService.plans.get("room-unmatched")).toBeUndefined()
			expect(contractService.spentReads).toEqual([])
		})
	})
})
