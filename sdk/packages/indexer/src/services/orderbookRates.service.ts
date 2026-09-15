// USD prices for tokens without a $1 peg, taken from the HyperFX orderbook's rates. The orderbook
// quotes every pair as quote per 1 base, so a token's USD price is its rate against a $1 stable,
// inverted when the stable is the base.
import Decimal from "decimal.js"

/**
 * USD per one whole `symbol`, from the orderbook's best rate against a $1 stable. Null when the
 * orderbook quotes no such pair.
 *
 * Mocked: nothing is fetched yet, so a token without a $1 peg stays unpriced and volume keeps only
 * its raw amount. It answers null rather than a made-up rate because whatever it returns is written
 * into cumulative USD volume, which is never recomputed.
 */
export async function fetchOrderbookUsdPrice(_symbol: string): Promise<Decimal | null> {
	return null
}
