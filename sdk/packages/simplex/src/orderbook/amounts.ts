import type { LimitOrderSide } from "@/data/types"

/**
 * The unit every amount and price crosses the orderbook boundary in.
 *
 * The orderbook normalises every token to 18 decimals so one book can hold
 * assets that disagree about decimals on chain. Raw amounts only appear inside
 * the signed UserOp, which the destination chain's tokens have to accept.
 */
export const ORDERBOOK_SCALE = 10n ** 18n

/** A normalised amount in the token's own units on `fillChain`. Truncates. */
export function toRaw(amount: bigint, decimals: number): bigint {
	return amount / 10n ** BigInt(18 - decimals)
}

/** A raw on-chain amount back at 1e18, the unit limit orders are kept in. */
export function toScaled(amount: bigint, decimals: number): bigint {
	return amount * 10n ** BigInt(18 - decimals)
}

function divCeil(numerator: bigint, denominator: bigint): bigint {
	return (numerator + denominator - 1n) / denominator
}

/**
 * The rate an operator's two amounts imply, as quote per 1 base at 1e18, and the
 * side of the book they trade.
 *
 * The operator states what they will take in and what they will pay out, so the
 * direction is the order rather than something chosen separately: taking the
 * base in and paying the quote out is a bid, the other way round an ask. A limit
 * order written this way can only ever price swaps going the same way.
 *
 * The rate is rounded in simplex's favour, the same direction
 * {@link signedAmounts} rounds, so rebuilding the op from it on a repost never
 * quotes better than the operator asked for.
 */
export function rateFrom(params: {
	base: string
	quote: string
	/** The symbol simplex takes in. */
	tokenIn: string
	/** What simplex takes in, at 1e18. */
	amountIn: bigint
	/** What simplex pays out, at 1e18. */
	amountOut: bigint
}): { side: LimitOrderSide; price: bigint } {
	const { base, quote, tokenIn, amountIn, amountOut } = params
	if (amountIn <= 0n || amountOut <= 0n) throw new Error("A limit order's amounts must both be greater than zero")

	if (tokenIn === base) {
		// Base in, quote out. A lower rate pays away less quote per base.
		return { side: "BID", price: (amountOut * ORDERBOOK_SCALE) / amountIn }
	}
	if (tokenIn === quote) {
		// Quote in, base out. A higher rate takes in more quote per base.
		return { side: "ASK", price: divCeil(amountIn * ORDERBOOK_SCALE, amountOut) }
	}
	throw new Error(`'${tokenIn}' is neither side of the ${base}/${quote} book`)
}

/**
 * The raw input and output a limit order signs for.
 *
 * `size` is the output simplex offers to pay and `price` is quote per 1 base,
 * both at 1e18. The input is rounded up so the rate the op actually carries is
 * never better for the taker than the operator's price: a bid ends up asking
 * for slightly more base, an ask for slightly more quote.
 */
export function signedAmounts(params: {
	side: LimitOrderSide
	size: bigint
	price: bigint
	baseDecimals: number
	quoteDecimals: number
}): { inputAmount: bigint; outputAmount: bigint } {
	const { side, size, price, baseDecimals, quoteDecimals } = params
	if (price <= 0n) throw new Error("A limit order's price must be greater than zero")

	const base = 10n ** BigInt(baseDecimals)
	const quote = 10n ** BigInt(quoteDecimals)

	if (side === "BID") {
		const outputAmount = toRaw(size, quoteDecimals)
		return { outputAmount, inputAmount: divCeil(outputAmount * base * ORDERBOOK_SCALE, price * quote) }
	}

	const outputAmount = toRaw(size, baseDecimals)
	return { outputAmount, inputAmount: divCeil(outputAmount * price * quote, base * ORDERBOOK_SCALE) }
}


/**
 * What a limit order pays for `inputAmount` at its own signed rate, at 1e18 but
 * quantised to the output token's raw unit on the fill chain.
 *
 * A bid receives the base and pays the quote, so its offer scales up with the
 * price; an ask is the other way round. Flooring keeps the payout at or inside
 * the rate simplex signed for, which is the same direction {@link signedAmounts}
 * rounds, and stops an offer being promised that the raw token cannot express.
 */
export function offerFor(params: {
	side: LimitOrderSide
	inputAmount: bigint
	price: bigint
	outputDecimals: number
}): bigint {
	const { side, inputAmount, price, outputDecimals } = params
	if (price <= 0n) throw new Error("A limit order's price must be greater than zero")

	const scaled = side === "BID" ? (inputAmount * price) / ORDERBOOK_SCALE : (inputAmount * ORDERBOOK_SCALE) / price
	const unit = 10n ** BigInt(18 - outputDecimals)
	return (scaled / unit) * unit
}
