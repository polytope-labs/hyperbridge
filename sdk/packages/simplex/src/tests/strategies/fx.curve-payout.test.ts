import { FXFiller, type TradingPair } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { AssetRegistry } from "@/config/asset-registry"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"
import { parseUnits } from "viem"

// Pins the fill amount `calculateProfitability` caches — the figure
// `prepareBidUserOp` signs into the bid verbatim — against the configured curve.
//
// Regression context: #1123 introduced `desiredOutput` for the `maxOrderSize`
// cap and took `targetOutput = min(policyMaxOutput, desiredOutput)`. The price
// gate right below guarantees `policyMaxOutput >= desiredOutput` for any leg
// that survives to a fill, so every fill silently paid the user's requested
// amount instead of the curve amount — and no test caught it, because nothing
// asserted the payout. These tests drive the evaluation with mocked chain
// access and assert the cached outputs for the three sizing outcomes: uncapped
// (pay the curve), capped (pay the capped slice's worth at the curve), and
// balance-limited (pay what the wallet covers).

const CHAIN = "EVM-97"
const STABLE = "0x1111111111111111111111111111111111111111" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

// Flat ask: the filler sells EXOTIC at 1500 per token0, at every size.
const FLAT_ASK = new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] })

// 100 token0 in; the order asks for 149,000 EXOTIC (rate 1490, below the
// curve's 1500) so the curve amount of 150,000 strictly exceeds the ask.
const INPUT_AMOUNT = parseUnits("100", 18)
const REQUESTED_OUTPUT = parseUnits("149000", 18)
const CURVE_OUTPUT = parseUnits("150000", 18)

const configService = {
	getUsdcAsset: () => STABLE,
	getUsdtAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
	getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
	getCNgnAsset: () => undefined,
	getMaxOverfillBps: () => 500n,
	getMaxConsecutiveClamps: () => 3,
	// No paymaster configured → paymasterReserveForToken contributes nothing.
	getCirclePaymasterAddress: () => undefined,
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
			clearPartialFill: (id: string) => partials.delete(id),
			setPartialFill: (id: string, value: boolean) => partials.set(id, value),
			getPartialFill: (id: string) => partials.get(id),
			setFundingPrepends: () => {},
			clearFundingPrepends: () => {},
		},
		outputs,
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

function makeFiller(options: {
	contractService: any
	balances: Record<string, bigint>
	maxOrderSize?: number
}): FXFiller {
	const registry = new AssetRegistry(configService, { EXOTIC: { [CHAIN]: EXOTIC } })
	const pairs: TradingPair[] = [
		{
			token0: "USDC",
			token1: "EXOTIC",
			...(options.maxOrderSize !== undefined ? { maxOrderSize: new Decimal(options.maxOrderSize) } : {}),
			askPricePolicy: FLAT_ASK,
		},
	]
	const signer = { address: SOLVER } as any
	return new FXFiller(
		signer,
		configService,
		makeClientManager(options.balances),
		options.contractService,
		pairs,
		registry,
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

describe("FXFiller curve payout", () => {
	it("pays the curve amount, not the user's requested amount", async () => {
		const contractService = makeEvalContractService()
		const filler = makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			maxOrderSize: 5000,
		})

		const profit = await filler.calculateProfitability(makeOrder("payout-full"))

		const cached = contractService.outputs.get("payout-full")
		expect(cached).toHaveLength(1)
		expect(cached![0].token).toBe(bytes20ToBytes32(EXOTIC))
		// 100 token0 at the flat ask of 1500 — the curve amount. The #1123
		// regression paid REQUESTED_OUTPUT (149,000) here instead.
		expect(cached![0].amount).toBe(CURVE_OUTPUT)
		expect(cached![0].amount).not.toBe(REQUESTED_OUTPUT)
		// Paying above the ask is a full fill (the gateway splits the excess),
		// and the fee surplus makes it score.
		expect(contractService.partials.get("payout-full")).toBe(false)
		expect(profit).toBeGreaterThan(0)
	})

	it("pays a capped leg the capped slice's worth at the curve, not the pro-rata ask", async () => {
		const contractService = makeEvalContractService()
		// Cap 40 of the 100 token0 notional: capFraction 0.4. The ration is
		// applied in token0 BEFORE the rate, so the payout is 40 × 1500 = 60,000 —
		// not the pro-rata ask of 149,000 × 0.4 = 59,600 (the pre-#1155 clamp),
		// and not the full ask.
		const filler = makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("1000000", 18) },
			maxOrderSize: 40,
		})

		await filler.calculateProfitability(makeOrder("payout-capped"))

		const cached = contractService.outputs.get("payout-capped")
		expect(cached).toHaveLength(1)
		expect(cached![0].amount).toBe(parseUnits("60000", 18))
		// The capped slice's worth is below the full ask, so this is an under-fill.
		expect(contractService.partials.get("payout-capped")).toBe(true)
	})

	it("pays what the wallet covers when balance-limited, still a full fill above the ask", async () => {
		const contractService = makeEvalContractService()
		// Wallet holds 149,500 — between the ask (149,000) and the curve amount
		// (150,000). The payout is capped by the balance, but it still clears the
		// ask, so the fill is full, not partial.
		const filler = makeFiller({
			contractService,
			balances: { [EXOTIC.toLowerCase()]: parseUnits("149500", 18) },
			maxOrderSize: 5000,
		})

		const profit = await filler.calculateProfitability(makeOrder("payout-balance"))

		const cached = contractService.outputs.get("payout-balance")
		expect(cached).toHaveLength(1)
		expect(cached![0].amount).toBe(parseUnits("149500", 18))
		expect(contractService.partials.get("payout-balance")).toBe(false)
		expect(profit).toBeGreaterThan(0)
	})
})
