import { FXFiller, type TradingPair } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
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

// Flat ask: the filler sells EXOTIC at 1500 per token0, at every size.
const FLAT_ASK = new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] })

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
	const inputs = new Map<string, TokenInfo[]>()
	const partials = new Map<string, boolean>()
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
			clearPartialFill: (id: string) => partials.delete(id),
			setPartialFill: (id: string, value: boolean) => partials.set(id, value),
			getPartialFill: (id: string) => partials.get(id),
			setFundingPrepends: () => {},
			clearFundingPrepends: () => {},
		},
		outputs,
		inputs,
		partials,
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
}): Promise<FXFiller> {
	const registry = new AssetRegistry(configService, { EXOTIC: { [CHAIN]: EXOTIC } })
	const pairs: TradingPair[] = [{ token0: "USDC", token1: "EXOTIC", askPricePolicy: FLAT_ASK }]
	const signer = { address: SOLVER } as any
	return new FXFiller(
		signer,
		configService,
		makeClientManager(options.balances),
		options.contractService,
		pairs,
		registry,
		{
			limitOrders: await limitOrderStore([
				{
					base: "USDC",
					quote: "EXOTIC",
					side: "BID",
					fillChain: CHAIN,
					price: "1500",
					size: options.offering ?? "1000000",
				},
			]),
		},
	)
}

/** Same-chain USDC→EXOTIC order (partial-fill eligible: no calldata, not cross-chain). */
function makeOrder(id: string): Order {
	const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT }]
	const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(EXOTIC), amount: REQUESTED_OUTPUT }]
	return {
		id,
		user: bytes20ToBytes32(SOLVER),
		source: CHAIN,
		destination: CHAIN,
		deadline: 0n,
		nonce: 0n,
		// Covers the (zero) execution cost with room to spare, so GATE 1 passes
		// and the evaluation reaches its cached-outputs answer.
		fees: parseUnits("1", 18),
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs,
		output: { beneficiary: bytes20ToBytes32(SOLVER), assets: outputs, call: "0x" as HexString },
	} as unknown as Order
}

describe("FXFiller limit order payout", () => {
	it("pays what the limit order offers, not the user's requested amount", async () => {
		const contractService = makeEvalContractService()
		const filler = await makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
		})

		const profit = await filler.calculateProfitability(makeOrder("payout-full"))

		const cached = contractService.outputs.get("payout-full")
		expect(cached).toHaveLength(1)
		expect(cached![0].token).toBe(bytes20ToBytes32(EXOTIC))
		// 100 in at the order's rate of 1500 — what the operator offered, which is
		// above the 149,000 asked for.
		expect(cached![0].amount).toBe(OFFERED_OUTPUT)
		expect(cached![0].amount).not.toBe(REQUESTED_OUTPUT)
		// Uncapped: the take is the leg's whole input.
		expect(contractService.inputs.get("payout-full")).toEqual([
			{ token: bytes20ToBytes32(STABLE), amount: INPUT_AMOUNT },
		])
		// Paying above the ask is a full fill (the gateway splits the excess),
		// and the fee surplus makes it score.
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

	it("pays what the wallet covers when balance-limited, still a full fill above the ask", async () => {
		const contractService = makeEvalContractService()
		// Wallet holds 149,500 — between the ask (149,000) and what the order
		// offered (150,000). The payout is capped by the balance, but it still clears the
		// ask, so the fill is full, not partial.
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
})
