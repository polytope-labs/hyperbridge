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

function divCeil(numerator: bigint, denominator: bigint): bigint {
	return (numerator + denominator - 1n) / denominator
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

