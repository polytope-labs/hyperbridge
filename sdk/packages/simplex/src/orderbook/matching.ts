import type { HexString } from "@hyperbridge/sdk"
import type { LimitOrder } from "@/data/types"
import { normalizeSymbol } from "@/config/asset-registry"
import { offerFor } from "./amounts"

/** Which symbols a limit order takes in and pays out, from the side it sits on. */
export function limitOrderLegs(order: LimitOrder): { input: string; output: string } {
	return order.side === "BID" ? { input: order.base, output: order.quote } : { input: order.quote, output: order.base }
}

/** An incoming order, reduced to what deciding a price needs. */
export interface IncomingOrder {
	source: string
	destination: string
	/** The input token's symbol on the source chain. */
	inputSymbol: string
	/** The output token's address on the destination chain, as the order asked for it. */
	outputToken: HexString
	/** The input left after the gateway took its protocol fee, at 1e18. */
	inputNet: bigint
	/** What the order asked to receive, at 1e18. */
	requestedOutput: bigint
	/** The output token's decimals on the destination chain. */
	outputDecimals: number
}

/** A limit order that can serve an incoming order, and on what terms. */
export interface LimitOrderMatch {
	order: LimitOrder
	/** What the order pays for `inputNet` at its own signed rate, at 1e18. */
	offer: bigint
	/** `remaining - reserved`: what is left to draw on, at 1e18. */
	available: bigint
	/** `min(offer, available)`: what simplex will actually pay. */
	payout: bigint
}

export function availableOn(order: LimitOrder): bigint {
	// Floored because a fill draws `remaining` down without touching what other
	// bids have reserved, so an order can owe more than it has left. That is
	// nothing to draw on, not capacity in reverse.
	const available = BigInt(order.remaining) - BigInt(order.reserved)
	return available > 0n ? available : 0n
}

/**
 * The limit orders an incoming order is priced against, in the order they should
 * be drawn on, or an empty list when none serves it.
 *
 * There is no fallback price: an order matching nothing is not filled, which is
 * the whole point of pricing from the operator's own resting orders rather than
 * from a curve that always has an answer.
 *
 * An order takes part only if its offer covers what the swapper asked for. The
 * gateway credits the swapper `take * T / I` for a bid's take and refuses a bid
 * whose own rate is below the order's (`RateBelowOrder`), so an order may only
 * take part where the swapper's rate is inside its own: `T / I <= price`, which
 * is exactly `offer >= requestedOutput`. Each bid then quotes the order's own
 * rate, and whatever it pays above the credit goes to the swapper and the
 * protocol.
 *
 * Several may be needed, because one that clears the rate may not have the depth.
 * The orderbook already quotes a swapper across every level that can fill the
 * trade together, so serving only the first would advertise depth and refuse to
 * meet it. What is paid in total is the ask, never the sum of the offers: each
 * order funds a slice, is drawn down by that slice, and receives that fraction of
 * the input, so every one of them settles at `T / I` too.
 *
 * They come best offer first: the order that would pay the most for this input.
 * That is the order each bid is built, held against its limit order and sent in.
 */
export function matchLimitOrders(
	orders: readonly LimitOrder[],
	incoming: IncomingOrder,
	resolve: (symbol: string, chain: string) => HexString | null,
	now: Date = new Date(),
): LimitOrderMatch[] {
	const qualifying = orders
		.filter((order) => serves(order, incoming, resolve, now))
		.map((order) => {
			const offer = offerFor({
				side: order.side,
				inputAmount: incoming.inputNet,
				price: BigInt(order.price),
				outputDecimals: incoming.outputDecimals,
			})
			const available = availableOn(order)
			return { order, offer, available, payout: offer < available ? offer : available }
		})
		// Below the ask is below the operator's rate, and an order with nothing left
		// to pay serves nobody whatever it quotes.
		.filter((candidate) => candidate.offer >= incoming.requestedOutput && candidate.payout > 0n)
		.sort(byBestOfferFirst)

	// Every qualifying order, best price first. Each one clears the ask on its own,
	// so each is a bid in its own right rather than a slice of a combined one: the
	// caller sends them in turn and the gateway clamps whichever lands against what
	// is still outstanding. Stopping once the ask was "covered" was the arithmetic
	// that billed one input to several orders at once.
	return qualifying
}

/** The first order to draw on, for callers that only need to know one exists. */
export function matchLimitOrder(
	orders: readonly LimitOrder[],
	incoming: IncomingOrder,
	resolve: (symbol: string, chain: string) => HexString | null,
	now: Date = new Date(),
): LimitOrderMatch | null {
	return matchLimitOrders(orders, incoming, resolve, now)[0] ?? null
}

/**
 * Largest offer first, then most left, then by id.
 *
 * Of two equal offers the deeper one goes first, since it is the likelier to cover
 * the swap on its own. The last comparison is what makes the order
 * reproducible: the same set of orders always goes out the same way.
 */
function byBestOfferFirst(a: LimitOrderMatch, b: LimitOrderMatch): number {
	if (a.offer !== b.offer) return a.offer > b.offer ? -1 : 1
	if (a.available !== b.available) return a.available > b.available ? -1 : 1
	return a.order.id < b.order.id ? -1 : 1
}

function serves(
	order: LimitOrder,
	incoming: IncomingOrder,
	resolve: (symbol: string, chain: string) => HexString | null,
	now: Date,
): boolean {
	if (order.status !== "open") return false
	if (order.expiresAt !== null && new Date(order.expiresAt) <= now) return false
	if (order.fillChain !== incoming.destination) return false

	// Symbols are compared case-insensitively: the book spells them as the
	// orderbook does ("cNGN") and the asset registry upper-cases them ("CNGN").
	const legs = limitOrderLegs(order)
	if (normalizeSymbol(legs.input) !== normalizeSymbol(incoming.inputSymbol)) return false

	// Compared by address on the destination, where the order named a real token,
	// and by symbol on the source, where the address belongs to another chain.
	const outputToken = resolve(legs.output, order.fillChain)
	if (!outputToken || outputToken.toLowerCase() !== incoming.outputToken.toLowerCase()) return false

	// Same-chain swaps ignore the declaration, as the orderbook does: there is no
	// source chain to accept or refuse.
	if (incoming.source === incoming.destination) return true
	return order.acceptedSources.includes(incoming.source)
}
