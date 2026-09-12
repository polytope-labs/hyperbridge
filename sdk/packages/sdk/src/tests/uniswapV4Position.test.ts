import {
	addressFromWord,
	asSigned,
	decodePoolAndPositionInfo,
	getAmountsForLiquidity,
	getSqrtRatioAtTick,
	positionAmountOfToken,
	positionTicks,
	word,
} from "@/protocols/intents/uniswap-v4-position"

const Q96 = 1n << 96n
const w = (value: bigint | string) =>
	(typeof value === "bigint" ? value.toString(16) : value.replace(/^0x/, "")).padStart(64, "0")
const addr = (a: string) => w(a)
const packInfo = (tickLower: number, tickUpper: number) =>
	(1n << 56n) | (BigInt.asUintN(24, BigInt(tickUpper)) << 32n) | (BigInt.asUintN(24, BigInt(tickLower)) << 8n) | 1n

const USDC = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const CNGN = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"

describe("getSqrtRatioAtTick", () => {
	// Pinned against Uniswap's TickMath. These are the values the pool itself computes, so a drift
	// here is a drift between what we report as withdrawable and what the pool would actually pay.
	it("matches TickMath at zero, the extremes, and either side of zero", () => {
		expect(getSqrtRatioAtTick(0)).toBe(79228162514264337593543950336n)
		expect(getSqrtRatioAtTick(0)).toBe(Q96)
		expect(getSqrtRatioAtTick(1)).toBe(79232123823359799118286999568n)
		expect(getSqrtRatioAtTick(-1)).toBe(79224201403219477170569942574n)
		expect(getSqrtRatioAtTick(-887272)).toBe(4295128739n)
		expect(getSqrtRatioAtTick(887272)).toBe(1461446703485210103287273052203988822378723970342n)
	})

	it("is monotonic across a sweep", () => {
		let previous = 0n
		for (let tick = -500_000; tick <= 500_000; tick += 9_973) {
			const ratio = getSqrtRatioAtTick(tick)
			expect(ratio).toBeGreaterThan(previous)
			previous = ratio
		}
	})

	it("rejects ticks outside the representable range", () => {
		expect(() => getSqrtRatioAtTick(887273)).toThrow()
		expect(() => getSqrtRatioAtTick(-887273)).toThrow()
		expect(() => getSqrtRatioAtTick(1.5)).toThrow()
	})
})

describe("position info decoding", () => {
	it("sign-extends packed negative ticks", () => {
		expect(positionTicks(packInfo(-120, -60))).toEqual({ tickLower: -120, tickUpper: -60 })
		expect(positionTicks(packInfo(-887220, 887220))).toEqual({ tickLower: -887220, tickUpper: 887220 })
		expect(positionTicks(packInfo(0, 60))).toEqual({ tickLower: 0, tickUpper: 60 })
		expect(asSigned(BigInt.asUintN(24, -5n), 24)).toBe(-5n)
	})

	it("reads the pool key and keeps its encoding verbatim for hashing", () => {
		const data = `0x${addr(CNGN)}${addr(USDC)}${w(1500n)}${w(30n)}${addr("0x" + "00".repeat(20))}${w(packInfo(-120, 120))}`
		const info = decodePoolAndPositionInfo(data)

		expect(info.currency0).toBe(CNGN)
		expect(info.currency1).toBe(USDC)
		expect(info.tickLower).toBe(-120)
		expect(info.tickUpper).toBe(120)
		// A static struct's abi.encode IS its concatenated words, so the pool id is keccak of exactly
		// this slice — re-encoding it ourselves is what would risk disagreeing with the chain.
		expect(info.poolKeyEncoded).toBe(`0x${data.slice(2, 2 + 5 * 64)}`)
		expect(info.poolKeyEncoded.length).toBe(2 + 5 * 64)
	})

	it("rejects truncated return data rather than reading zeros", () => {
		expect(() => decodePoolAndPositionInfo(`0x${w(1n).repeat(5)}`)).toThrow()
		expect(() => word("0x", 0)).toThrow()
	})

	it("reads addresses out of the low bytes of a word", () => {
		expect(addressFromWord(`0x${addr(USDC)}`, 0)).toBe(USDC)
	})
})

describe("getAmountsForLiquidity", () => {
	const lower = getSqrtRatioAtTick(-60)
	const upper = getSqrtRatioAtTick(60)
	const liquidity = 10n ** 18n

	// A position out of range holds exactly one token — which is why a declared position can back
	// one leg of a pair and contribute nothing at all to the other.
	it("is all token0 below the range and all token1 above it", () => {
		const below = getAmountsForLiquidity({
			sqrtPriceX96: getSqrtRatioAtTick(-600),
			sqrtRatioAX96: lower,
			sqrtRatioBX96: upper,
			liquidity,
		})
		expect(below.amount1).toBe(0n)
		expect(below.amount0).toBeGreaterThan(0n)

		const above = getAmountsForLiquidity({
			sqrtPriceX96: getSqrtRatioAtTick(600),
			sqrtRatioAX96: lower,
			sqrtRatioBX96: upper,
			liquidity,
		})
		expect(above.amount0).toBe(0n)
		expect(above.amount1).toBeGreaterThan(0n)
	})

	it("holds both sides in range, split by where the price sits", () => {
		const mid = getAmountsForLiquidity({
			sqrtPriceX96: getSqrtRatioAtTick(0),
			sqrtRatioAX96: lower,
			sqrtRatioBX96: upper,
			liquidity,
		})
		expect(mid.amount0).toBeGreaterThan(0n)
		expect(mid.amount1).toBeGreaterThan(0n)
		// A symmetric range priced at its midpoint is near-balanced in a 1:1 pool.
		const ratio = Number(mid.amount0) / Number(mid.amount1)
		expect(ratio).toBeGreaterThan(0.99)
		expect(ratio).toBeLessThan(1.01)
	})

	it("does not care which order the bounds are given in", () => {
		const price = getSqrtRatioAtTick(10)
		expect(
			getAmountsForLiquidity({ sqrtPriceX96: price, sqrtRatioAX96: upper, sqrtRatioBX96: lower, liquidity }),
		).toEqual(
			getAmountsForLiquidity({ sqrtPriceX96: price, sqrtRatioAX96: lower, sqrtRatioBX96: upper, liquidity }),
		)
	})

	it("scales linearly with liquidity", () => {
		const one = getAmountsForLiquidity({
			sqrtPriceX96: getSqrtRatioAtTick(0),
			sqrtRatioAX96: lower,
			sqrtRatioBX96: upper,
			liquidity,
		})
		const ten = getAmountsForLiquidity({
			sqrtPriceX96: getSqrtRatioAtTick(0),
			sqrtRatioAX96: lower,
			sqrtRatioBX96: upper,
			liquidity: liquidity * 10n,
		})
		expect(ten.amount0 / one.amount0).toBe(10n)
		expect(ten.amount1 / one.amount1).toBe(10n)
	})
})

describe("positionAmountOfToken", () => {
	const info = decodePoolAndPositionInfo(
		`0x${addr(CNGN)}${addr(USDC)}${w(1500n)}${w(30n)}${addr("0x" + "00".repeat(20))}${w(packInfo(-60, 60))}`,
	)
	const inRange = { info, liquidity: 10n ** 18n, sqrtPriceX96: getSqrtRatioAtTick(0) }

	it("returns the side matching the requested token", () => {
		const amounts = getAmountsForLiquidity({
			sqrtPriceX96: inRange.sqrtPriceX96,
			sqrtRatioAX96: getSqrtRatioAtTick(-60),
			sqrtRatioBX96: getSqrtRatioAtTick(60),
			liquidity: inRange.liquidity,
		})
		expect(positionAmountOfToken({ ...inRange, outputToken: CNGN })).toBe(amounts.amount0)
		expect(positionAmountOfToken({ ...inRange, outputToken: USDC.toUpperCase() })).toBe(amounts.amount1)
	})

	// A position in some other pool is not capacity for this leg, and saying so must not require
	// the caller to have checked first.
	it("is zero for a token the pool does not hold, and for empty or unpriced positions", () => {
		expect(positionAmountOfToken({ ...inRange, outputToken: `0x${"11".repeat(20)}` })).toBe(0n)
		expect(positionAmountOfToken({ ...inRange, liquidity: 0n, outputToken: CNGN })).toBe(0n)
		expect(positionAmountOfToken({ ...inRange, sqrtPriceX96: 0n, outputToken: CNGN })).toBe(0n)
	})

	it("is zero for the side a price move has emptied", () => {
		const wellAbove = { ...inRange, sqrtPriceX96: getSqrtRatioAtTick(600) }
		expect(positionAmountOfToken({ ...wellAbove, outputToken: CNGN })).toBe(0n)
		expect(positionAmountOfToken({ ...wellAbove, outputToken: USDC })).toBeGreaterThan(0n)
	})
})
