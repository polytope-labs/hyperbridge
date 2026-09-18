import { FXFiller, type TradingPair } from "@/strategies/fx"
import { AssetRegistry } from "@/config/asset-registry"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { parseUnits } from "viem"
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
function makeEvalContractService(): any {
	const classifications = new Map<string, unknown>()
	const outputs = new Map<string, TokenInfo[]>()
	const partials = new Map<string, boolean>()
	const bidPlans = new Map<string, unknown>()
	return {
		getTokenDecimals: async () => 18,
		getFeeTokenWithDecimals: async () => ({ address: STABLE, decimals: 18 }),
		estimateGasFillPost: async () => ({
			totalCostInSourceFeeToken: 0n,
			relayerFeeInSourceFeeToken: 0n,
			dispatchFee: 0n,
		}),
		// No prior partial fills on-chain: an under-fill stays eligible.
		partialFillsFor: async () => [0n],
		cacheService: {
			getPairClassifications: (id: string) => classifications.get(id),
			setPairClassifications: (id: string, pairs: unknown) => classifications.set(id, pairs),
			getFillerOutputs: (id: string) => outputs.get(id),
			setFillerOutputs: (id: string, value: TokenInfo[]) => outputs.set(id, value),
			setMatchedLimitOrder: () => {},
			setBidPlans: (id: string, plans: unknown) => bidPlans.set(id, plans),
			getBidPlans: (id: string) => bidPlans.get(id) ?? [],
			clearBidPlans: (id: string) => bidPlans.delete(id),
			clearPartialFill: (id: string) => partials.delete(id),
			setPartialFill: (id: string, value: boolean) => partials.set(id, value),
			getPartialFill: (id: string) => partials.get(id),
			setFundingPrepends: () => {},
			clearFundingPrepends: () => {},
		},
		outputs,
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
	book?: { price: string; size: string; id?: string }[]
}): Promise<FXFiller> {
	const registry = new AssetRegistry(configService, { EXOTIC: { [CHAIN]: EXOTIC } })
	const pairs: TradingPair[] = [{ token0: "USDC", token1: "EXOTIC" }]
	const signer = { address: SOLVER } as any
	return new FXFiller(
		signer,
		configService,
		makeClientManager(options.balances),
		options.contractService,
		pairs,
		registry,
		{
			limitOrders: await limitOrderStore(
				(options.book ?? [{ price: "1500", size: options.offering ?? "1000000" }]).map((order) => ({
					base: "USDC",
					quote: "EXOTIC",
					side: "BID" as const,
					fillChain: CHAIN,
					price: order.price,
					size: order.size,
					acceptedSources: [CHAIN, "EVM-1"],
					id: order.id,
				})),
			),
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
		expect(plans[0].limitOrderId).toBe("tight")
	})

	it("sends one bid per limit order, each priced against the whole input", async () => {
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
		expect(plans.map((plan) => plan.limitOrderId)).toEqual(["tight", "wide"])
		// Each bids the ask: both offers clear it, and the bid is never above it.
		for (const plan of plans) {
			expect(plan.fillerOutputs[0].amount).toBe(REQUESTED_OUTPUT)
		}
	})
	it("bids the ask, not the more the limit order was willing to pay", async () => {
		// The gateway sets `fillAmount = totalRequired` on a full fill and splits
		// everything above it between the beneficiary and the protocol, debiting the
		// solver the whole bid. The escrow released is the same either way, so the
		// difference between the offer and the ask is margin kept by not bidding it.
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
		expect(cached![0].amount).toBe(REQUESTED_OUTPUT)
		expect(cached![0].amount).toBeLessThan(OFFERED_OUTPUT)
		// Meeting the ask in full is a full fill, and the margin left makes it score.
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
		// offered (150,000). The bid is the ask either way, so the balance never
		// binds and the fill is full.
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("149500", 18) },
		})

		const profit = await filler.calculateProfitability(makeOrder("payout-balance"))

		const cached = contractService.outputs.get("payout-balance")
		expect(cached).toHaveLength(1)
		expect(cached![0].amount).toBe(REQUESTED_OUTPUT)
		expect(contractService.partials.get("payout-balance")).toBe(false)
		expect(profit).toBeGreaterThan(0)
	})
})
