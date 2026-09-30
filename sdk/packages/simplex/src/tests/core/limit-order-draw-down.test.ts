import { describe, expect, it } from "vitest"
import { bytes20ToBytes32, type HexString, type Order } from "@hyperbridge/sdk"
import { AssetRegistry } from "@/config/asset-registry"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { budgetFor, toRaw, toScaled } from "@/orderbook/amounts"
import { CacheService } from "@/services/CacheService"
import { FXFiller } from "@/strategies/fx"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { limitOrderStore } from "../helpers/limit-orders"

/**
 * A limit order's budget is tallied on the fill chain in the output token's own units,
 * from what the gateway charged each fill. The store works the order down from the fill
 * events instead, and sizes the next bid from what it has left. The two have to agree to
 * the unit: a store that has drawn down less than the chain counted offers more than the
 * budget has room for, and that bid reverts every time it is rebuilt.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const LIMIT_ORDER = "3f2b8c1e-9d4a-4f6b-8a7c-5e1d2c3b4a59"
const CNGN = "0xCCCC00000000000000000000000000000000CCCC" as HexString
const USDC = "0xDDDD00000000000000000000000000000000DDDD" as HexString
const INPUT_DECIMALS = 6

const ceilDiv = (numerator: bigint, denominator: bigint) => (numerator + denominator - 1n) / denominator
const min = (a: bigint, b: bigint) => (a < b ? a : b)
const max = (a: bigint, b: bigint) => (a > b ? a : b)

/** `_cumulativeReleased` in the gateway. */
function cumulativeReleased(escrow: bigint, filled: bigint, required: bigint): bigint {
	return filled >= required ? escrow : (escrow * filled) / required
}

/** `_priceLeg` in the gateway: what one bid is credited, releases and is charged, in raw units. */
function priceLeg(leg: { escrow: bigint; required: bigint; previousCredit: bigint }, quoted: bigint, offered: bigint) {
	const { escrow, required, previousCredit } = leg
	if (ceilDiv(quoted * required, escrow) > offered) throw new Error("RateBelowOrder")
	const credited = min((quoted * required) / escrow, required - previousCredit)
	const released =
		cumulativeReleased(escrow, previousCredit + credited, required) -
		cumulativeReleased(escrow, previousCredit, required)
	if (credited === 0n || released === 0n) throw new Error("RateFillTooSmall")
	return { credited, released, paid: max(credited, ceilDiv(offered * released, quoted)) }
}

async function build(decimals: number, size: string) {
	const data = new MemoryDataStore()
	const limitOrders = await limitOrderStore([
		{ id: LIMIT_ORDER, base: "USDC", quote: "CNGN", side: "BID", fillChain: "EVM-8453", price: "1.0003", size },
	])
	const filler = new IntentFiller(
		[],
		[],
		{},
		{ getHyperbridgeWsUrl: () => undefined, getSubstratePrivateKey: () => undefined } as any,
		{} as any,
		{} as any,
		{ address: OUR_ADDRESS } as any,
		{ orders: stubOrderScanner() },
		undefined,
		data.bids,
		limitOrders,
	) as any
	filler.assetRegistry = { getAddress: () => CNGN }
	filler.contractService = { getTokenDecimals: async () => decimals }
	filler.limitOrderService = { resize: async () => null }

	const order = (await limitOrders.get(LIMIT_ORDER))!
	const { cap } = budgetFor(order, CNGN, decimals)
	const remaining = async () => BigInt((await limitOrders.get(LIMIT_ORDER))!.remaining)
	let tally = 0n
	let fills = 0

	/**
	 * One bid of ours landing on a swap that others have part filled: held, charged by the
	 * gateway, tallied against the budget, and settled from the fill event.
	 */
	const fill = async (
		leg: { escrow: bigint; required: bigint; previousCredit: bigint },
		quoted: bigint,
		offered: bigint,
		/** What the bid held against the limit order, when the strategy said so itself. */
		held: bigint = toScaled(offered, decimals),
	) => {
		const commitment = `0x${(++fills).toString(16).padStart(64, "0")}` as HexString
		const amount = held.toString()
		expect(await limitOrders.reserve(LIMIT_ORDER, amount)).toBe(true)
		await data.bids.store({
			commitment,
			bid: `0x${"b".repeat(63)}${fills}`,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount, take: quoted.toString() }],
		})

		const charge = priceLeg(leg, quoted, offered)
		if (tally + charge.paid > cap) throw new Error(`LimitOrderExceeded: total ${tally + charge.paid} > cap ${cap}`)
		tally += charge.paid

		await filler.settleFilledLimitOrder(
			commitment,
			8453,
			[{ token: CNGN, amount: charge.credited }],
			[{ token: USDC, amount: charge.released }],
			true,
		)
		return charge
	}

	return { cap, fill, remaining, limitOrders, bids: data.bids, filler, tally: () => tally, size: BigInt(order.size) }
}

/**
 * What the strategy signs and holds for a swap of 250 USDC asking 249.5, when the wallet
 * holds `balance` of the output token: less than the limit order's rate offers, so the
 * balance is what sizes the bid.
 */
async function balanceLimitedBid(ctx: Awaited<ReturnType<typeof build>>, decimals: number, balance: bigint) {
	const chain = "EVM-8453"
	const cache = new CacheService()
	const configService = {
		getUsdcAsset: () => USDC.toLowerCase(),
		getUsdtAsset: () => "0x0000000000000000000000000000000000000000",
		getDaiAsset: () => "0x0000000000000000000000000000000000000000",
		getCNgnAsset: () => undefined,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
		getSimplexPaymasterAddress: () => undefined,
	} as any
	const client = {
		chain: { blockTime: 2000 },
		getBlock: async () => ({ number: 100n, timestamp: 1_000_000n }),
		getBalance: async () => 0n,
		readContract: async () => balance,
	}
	const strategy = new FXFiller(
		{ address: OUR_ADDRESS } as any,
		configService,
		{ getPublicClient: () => client } as any,
		{
			getTokenDecimals: async (token: string) =>
				token.toLowerCase() === CNGN.toLowerCase() ? decimals : INPUT_DECIMALS,
			getFeeTokenWithDecimals: async () => ({ address: USDC, decimals: INPUT_DECIMALS }),
			estimateGasFillPost: async () => ({
				totalCostInSourceFeeToken: 0n,
				relayerFeeInSourceFeeToken: 0n,
				dispatchFee: 0n,
			}),
			partialFillsFor: async () => [0n],
			limitOrderSpent: async () => ctx.tally(),
			cacheService: cache,
		} as any,
		[{ token0: "USDC", token1: "CNGN" }],
		new AssetRegistry(configService, { CNGN: { [chain]: CNGN.toLowerCase() as HexString } }),
		{ limitOrders: ctx.limitOrders },
	)
	const order = {
		id: "balance-limited",
		user: bytes20ToBytes32(OUR_ADDRESS),
		source: chain,
		destination: chain,
		deadline: 0n,
		nonce: 0n,
		fees: 10n ** BigInt(INPUT_DECIMALS),
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: bytes20ToBytes32(USDC), amount: 250n * 10n ** BigInt(INPUT_DECIMALS) }],
		output: {
			beneficiary: bytes20ToBytes32(OUR_ADDRESS),
			assets: [{ token: bytes20ToBytes32(CNGN), amount: (2495n * 10n ** BigInt(decimals)) / 10n }],
			call: "0x",
		},
	} as unknown as Order

	await strategy.calculateProfitability(order)

	const [plan] = cache.getBidPlans(order.id!)
	return { offered: plan.fillerOutputs[0].amount, quoted: plan.fillerInputs[0].amount, held: plan.payout }
}

/**
 * Four bids of 250 USDC, each landing on a swap with only part of its ask still open, so
 * the gateway releases part of the take and charges that part at the bid's rate: a charge
 * that never lands on a whole unit of the output token. The bids offer 1.0003 unless the
 * case brings its own.
 */
async function clampedFills(
	ctx: Awaited<ReturnType<typeof build>>,
	decimals: number,
	bid?: { offered: bigint; quoted: bigint; held: bigint },
) {
	const unit = 10n ** BigInt(decimals)
	const quoted = bid?.quoted ?? 250n * 10n ** BigInt(INPUT_DECIMALS)
	const offered = bid?.offered ?? (2500750n * unit) / 10000n
	// Each swap asks 249.5 for its 250 and has this much of the ask left to fill.
	const required = (2495n * unit) / 10n
	const open = [159_123_457n, 159_654_321n, 158_777_779n, 160_333_331n].map((left) => (left * unit) / 10n ** 6n)

	for (const left of open) {
		const charge = await ctx.fill(
			{ escrow: quoted, required, previousCredit: required - left },
			quoted,
			offered,
			bid?.held,
		)
		expect(charge.released).toBeLessThan(quoted)
		expect(charge.paid).toBeGreaterThan(charge.credited)
		// What the store would bid next never exceeds the room the budget has left.
		expect(toRaw(await ctx.remaining(), decimals)).toBeLessThanOrEqual(ctx.cap - ctx.tally())
	}
}

describe("a limit order's draw-down against its on-chain budget", () => {
	for (const decimals of [6, 18]) {
		it(`draws down exactly what the gateway charged a clamped fill (${decimals} decimals)`, async () => {
			const ctx = await build(decimals, "1000")
			await clampedFills(ctx, decimals)

			expect(ctx.size - (await ctx.remaining())).toBe(toScaled(ctx.tally(), decimals))

			// A bid that takes everything the store has left, as the strategy sizes it, fits.
			const offered = toRaw(await ctx.remaining(), decimals)
			expect(offered).toBeGreaterThan(0n)
			expect(ctx.tally() + offered).toBeLessThanOrEqual(ctx.cap)
			const quoted = 400n * 10n ** BigInt(INPUT_DECIMALS)
			const required = (offered * 9990n) / 10000n
			const charge = await ctx.fill({ escrow: quoted, required, previousCredit: 0n }, quoted, offered)

			expect(charge.paid).toBe(offered)
			expect(ctx.tally()).toBe(ctx.cap)
			expect(await ctx.remaining()).toBe(0n)
		})
	}

	it("keeps a size finer than the token as dust beneath the budget", async () => {
		const ctx = await build(6, "1000.0000005")
		await clampedFills(ctx, 6)

		const offered = toRaw(await ctx.remaining(), 6)
		expect(ctx.tally() + offered).toBe(ctx.cap)
		expect((await ctx.remaining()) - toScaled(offered, 6)).toBe(5n * 10n ** 11n)
	})

	it("never draws down less than the gateway charged a bid signed in whole 1e18 units of a finer token", async () => {
		// The store is the coarser side here, so it cannot match the charge to the unit.
		// Rounded up, it can only ever think it has less left than the budget does.
		const ctx = await build(20, "1000")
		await clampedFills(ctx, 20)

		const drawn = ctx.size - (await ctx.remaining())
		expect(toRaw(drawn, 20)).toBeGreaterThanOrEqual(ctx.tally())
		expect(toRaw(drawn, 20) - ctx.tally()).toBeLessThan(4n * 100n)
	})

	it("keeps pace with the budget when the wallet's balance sizes a bid on a finer token", async () => {
		// A balance is any raw figure, and the hold beside it is kept at 1e18. A bid that
		// signed the balance as it stood would be charged at a rate its hold cannot express.
		const ctx = await build(20, "1000")
		const bid = await balanceLimitedBid(ctx, 20, 2499n * 10n ** 19n + 99n)
		expect(bid.offered).toBeLessThan((2500750n * 10n ** 20n) / 10000n)

		await clampedFills(ctx, 20, bid)

		const offered = toRaw(await ctx.remaining(), 20)
		expect(offered).toBeGreaterThan(0n)
		expect(ctx.tally() + offered).toBeLessThanOrEqual(ctx.cap)
	})

	it("floors at nothing left when a fill is charged more than the store still had", async () => {
		// A bid sized before another fill landed can be charged past what is left by the
		// time it settles. The budget is what refuses that on chain; the store must not
		// answer it with a negative remainder, which would read as room.
		const ctx = await build(6, "1000")
		const quoted = 10n ** 6n
		const amount = toScaled(1n, 6).toString()
		expect(await ctx.limitOrders.reserve(LIMIT_ORDER, amount)).toBe(true)
		await ctx.bids.store({
			commitment: "0xstale" as HexString,
			bid: "0xstalebid",
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount, take: quoted.toString() }],
		})
		// Other fills leave half a unit of the token; this one is charged a whole unit.
		await ctx.limitOrders.drawDown(LIMIT_ORDER, (ctx.size - 5n * 10n ** 11n).toString())

		await ctx.filler.settleFilledLimitOrder(
			"0xstale",
			8453,
			[{ token: CNGN, amount: 1n }],
			[{ token: USDC, amount: quoted }],
			true,
		)

		expect(await ctx.remaining()).toBe(0n)
	})
})
