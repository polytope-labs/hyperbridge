// Uniswap V4 position valuation, in plain bigint arithmetic.
//
// Lives apart from the aggregation and performs no I/O: the indexer runs inside the SubQuery VM2
// sandbox, which has neither viem nor the Uniswap SDK, and this is the one place in the phantom
// pipeline where a silent arithmetic slip would inflate reported depth rather than merely lose it.
// Keeping it pure means every branch is exercisable from a table of known values.
//
// A position's withdrawable amount of a given token is not stored on-chain. It is derived from the
// position's liquidity, its tick range, and the pool's current price — so a position that has moved
// out of range holds exactly one of its two tokens, and the same liquidity is worth different
// amounts of each token at different prices.

export type HexString = `0x${string}`

/** Q96 fixed-point scale used throughout Uniswap's price math. */
const Q96 = 1n << 96n

const MIN_TICK = -887272
const MAX_TICK = 887272

/** Reads a 32-byte word out of `0x`-prefixed return data as an unsigned bigint. */
export function word(data: string, index: number): bigint {
	const hex = data.startsWith("0x") ? data.slice(2) : data
	const start = index * 64
	if (hex.length < start + 64) throw new Error(`return data too short for word ${index}`)
	return BigInt(`0x${hex.slice(start, start + 64)}`)
}

/** Reinterprets the low `bits` of an unsigned word as a two's-complement signed value. */
export function asSigned(value: bigint, bits: number): bigint {
	const width = 1n << BigInt(bits)
	const masked = value & (width - 1n)
	return masked >= width >> 1n ? masked - width : masked
}

/** The 20-byte address in the low bytes of a 32-byte word, lowercased. */
export function addressFromWord(data: string, index: number): HexString {
	const hex = data.startsWith("0x") ? data.slice(2) : data
	return `0x${hex.slice(index * 64 + 24, (index + 1) * 64)}`.toLowerCase() as HexString
}

/**
 * `sqrt(1.0001^tick) * 2^96`, matching Uniswap's TickMath.getSqrtRatioAtTick bit for bit.
 *
 * The constants are the fixed-point square roots of 1.0001^(2^i); multiplying the ones selected by
 * the set bits of |tick| reconstructs the ratio by binary exponentiation without ever leaving
 * integer arithmetic. Reproduced rather than approximated with floating point, because the result
 * feeds an amount that must agree with what the pool would actually pay out.
 */
export function getSqrtRatioAtTick(tick: number): bigint {
	if (!Number.isInteger(tick) || tick < MIN_TICK || tick > MAX_TICK) {
		throw new Error(`tick out of range: ${tick}`)
	}
	const absTick = BigInt(Math.abs(tick))

	let ratio = (absTick & 0x1n) !== 0n ? 0xfffcb933bd6fad37aa2d162d1a594001n : 0x100000000000000000000000000000000n
	const factors: [bigint, bigint][] = [
		[0x2n, 0xfff97272373d413259a46990580e213an],
		[0x4n, 0xfff2e50f5f656932ef12357cf3c7fdccn],
		[0x8n, 0xffe5caca7e10e4e61c3624eaa0941cd0n],
		[0x10n, 0xffcb9843d60f6159c9db58835c926644n],
		[0x20n, 0xff973b41fa98c081472e6896dfb254c0n],
		[0x40n, 0xff2ea16466c96a3843ec78b326b52861n],
		[0x80n, 0xfe5dee046a99a2a811c461f1969c3053n],
		[0x100n, 0xfcbe86c7900a88aedcffc83b479aa3a4n],
		[0x200n, 0xf987a7253ac413176f2b074cf7815e54n],
		[0x400n, 0xf3392b0822b70005940c7a398e4b70f3n],
		[0x800n, 0xe7159475a2c29b7443b29c7fa6e889d9n],
		[0x1000n, 0xd097f3bdfd2022b8845ad8f792aa5825n],
		[0x2000n, 0xa9f746462d870fdf8a65dc1f90e061e5n],
		[0x4000n, 0x70d869a156d2a1b890bb3df62baf32f7n],
		[0x8000n, 0x31be135f97d08fd981231505542fcfa6n],
		[0x10000n, 0x9aa508b5b7a84e1c677de54f3e99bc9n],
		[0x20000n, 0x5d6af8dedb81196699c329225ee604n],
		[0x40000n, 0x2216e584f5fa1ea926041bedfe98n],
		[0x80000n, 0x48a170391f7dc42444e8fa2n],
	]
	for (const [bit, factor] of factors) {
		if ((absTick & bit) !== 0n) ratio = (ratio * factor) >> 128n
	}

	if (tick > 0) ratio = (1n << 256n) / ratio

	// Round up to Q96, as TickMath does, so the ratio never understates the price.
	return (ratio >> 32n) + (ratio % (1n << 32n) === 0n ? 0n : 1n)
}

/** The tick bounds packed into a PositionInfo word: `poolId(200) ‖ tickUpper(24) ‖ tickLower(24) ‖ hasSubscriber(8)`. */
export function positionTicks(info: bigint): { tickLower: number; tickUpper: number } {
	return {
		tickLower: Number(asSigned(info >> 8n, 24)),
		tickUpper: Number(asSigned(info >> 32n, 24)),
	}
}

/**
 * The token0/token1 amounts `liquidity` is worth at the current price, mirroring
 * LiquidityAmounts.getAmountsForLiquidity.
 *
 * Below its range a position is entirely token0, above it entirely token1, and inside it holds
 * both — which is why a declared position's contribution to a leg depends on which side that leg
 * pays out, and why it can legitimately be zero.
 */
export function getAmountsForLiquidity(params: {
	sqrtPriceX96: bigint
	sqrtRatioAX96: bigint
	sqrtRatioBX96: bigint
	liquidity: bigint
}): { amount0: bigint; amount1: bigint } {
	let { sqrtRatioAX96, sqrtRatioBX96 } = params
	const { sqrtPriceX96, liquidity } = params
	if (sqrtRatioAX96 > sqrtRatioBX96) [sqrtRatioAX96, sqrtRatioBX96] = [sqrtRatioBX96, sqrtRatioAX96]

	const amount0For = (from: bigint, to: bigint) => (liquidity * Q96 * (to - from)) / to / from
	const amount1For = (from: bigint, to: bigint) => (liquidity * (to - from)) / Q96

	if (sqrtPriceX96 <= sqrtRatioAX96) {
		return { amount0: amount0For(sqrtRatioAX96, sqrtRatioBX96), amount1: 0n }
	}
	if (sqrtPriceX96 < sqrtRatioBX96) {
		return {
			amount0: amount0For(sqrtPriceX96, sqrtRatioBX96),
			amount1: amount1For(sqrtRatioAX96, sqrtPriceX96),
		}
	}
	return { amount0: 0n, amount1: amount1For(sqrtRatioAX96, sqrtRatioBX96) }
}

/** A decoded `getPoolAndPositionInfo` return: the pool's key words plus the packed position info. */
export interface PoolAndPositionInfo {
	currency0: HexString
	currency1: HexString
	/**
	 * The ABI-encoded PoolKey exactly as returned, i.e. the first five words. A static struct's
	 * abi.encode IS its concatenated words, so `keccak256` of this slice is the pool id — no
	 * re-encoding, and no chance of the re-encoding disagreeing with what the chain hashed.
	 */
	poolKeyEncoded: HexString
	tickLower: number
	tickUpper: number
}

/** Decodes `getPoolAndPositionInfo(uint256)` return data (five static PoolKey words + info). */
export function decodePoolAndPositionInfo(data: string): PoolAndPositionInfo {
	const hex = data.startsWith("0x") ? data.slice(2) : data
	if (hex.length < 6 * 64) throw new Error("getPoolAndPositionInfo returned too little data")
	const { tickLower, tickUpper } = positionTicks(word(data, 5))
	return {
		currency0: addressFromWord(data, 0),
		currency1: addressFromWord(data, 1),
		poolKeyEncoded: `0x${hex.slice(0, 5 * 64)}` as HexString,
		tickLower,
		tickUpper,
	}
}

/**
 * How much of `outputToken` a position is currently worth. Zero when the token is not one of the
 * pool's currencies, or when the price has moved the position entirely onto the other side — both
 * ordinary outcomes, not errors.
 */
export function positionAmountOfToken(params: {
	info: PoolAndPositionInfo
	liquidity: bigint
	sqrtPriceX96: bigint
	outputToken: string
}): bigint {
	const { info, liquidity, sqrtPriceX96 } = params
	const outputToken = params.outputToken.toLowerCase()
	if (outputToken !== info.currency0 && outputToken !== info.currency1) return 0n
	if (liquidity <= 0n || sqrtPriceX96 <= 0n) return 0n

	const { amount0, amount1 } = getAmountsForLiquidity({
		sqrtPriceX96,
		sqrtRatioAX96: getSqrtRatioAtTick(info.tickLower),
		sqrtRatioBX96: getSqrtRatioAtTick(info.tickUpper),
		liquidity,
	})
	return outputToken === info.currency0 ? amount0 : amount1
}
