import type { HexString } from "@hyperbridge/sdk"
import type { LimitOrder } from "@/data/types"
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
 * The limit orders an incoming order is priced against, best payout first, or an
 * empty list when none serves it.
 *
 * There is no fallback price: an order matching nothing is not filled, which is
 * the whole point of pricing from the operator's own resting orders rather than
 * from a curve that always has an answer.
 *
 * Several orders may serve one swap. The orderbook already quotes a same-chain
 * swapper "the clearing price, the best at which the orders at it or better can
 * fill the trade together", so honouring only the best-priced one advertises
 * depth the operator has and then refuses to meet it. Levels are walked best
 * first and stop as soon as the ask is covered, so a swap one order covers still
 * draws on one.
 *
 * What each pays is `min(offer, remaining - reserved)`, not the offer alone: an
 * order quoting a wonderful rate with almost nothing behind it would otherwise
 * crowd out one that can actually cover the swap.
 *
 * An order whose offer falls short of what the swapper asked for is still a
 * match. Whether a shortfall can be filled at all is the caller's rule, not the
 * matcher's: a cross-chain order reverts on any under-fill, while a same-chain
 * one may fill partially, and that holds whether the shortfall comes from the
 * price or from what the orders have left.
 */
export function matchLimitOrders(
	orders: readonly LimitOrder[],
	incoming: IncomingOrder,
	resolve: (symbol: string, chain: string) => HexString | null,
	now: Date = new Date(),
): LimitOrderMatch[] {
	const candidates = orders
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
		// An order with nothing left to pay serves nobody, whatever it quotes.
		.filter((candidate) => candidate.payout > 0n)
		.sort(byPayoutThenAvailable)

	// The best candidate is always taken, then levels are added while the ask is
	// still short. An order asking for nothing, which is how a quote reaches the
	// engine, still draws on the one that prices it.
	const taken: LimitOrderMatch[] = []
	let covered = 0n
	for (const candidate of candidates) {
		taken.push(candidate)
		covered += candidate.payout
		if (covered >= incoming.requestedOutput) break
	}
	return taken
}

/** The best single match, for callers that only need to know whether one exists. */
export function matchLimitOrder(
	orders: readonly LimitOrder[],
	incoming: IncomingOrder,
	resolve: (symbol: string, chain: string) => HexString | null,
	now: Date = new Date(),
): LimitOrderMatch | null {
	return matchLimitOrders(orders, incoming, resolve, now)[0] ?? null
}

/**
 * Best payout first, ties to the one with more left, then by id.
 *
 * Deterministic to the last comparison, because the draw-down after a fill walks
 * these orders in the same sequence and has to reach the same answer.
 */
function byPayoutThenAvailable(a: LimitOrderMatch, b: LimitOrderMatch): number {
	if (a.payout !== b.payout) return a.payout > b.payout ? -1 : 1
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

	const legs = limitOrderLegs(order)
	if (legs.input !== incoming.inputSymbol) return false

	// Compared by address on the destination, where the order named a real token,
	// and by symbol on the source, where the address belongs to another chain.
	const outputToken = resolve(legs.output, order.fillChain)
	if (!outputToken || outputToken.toLowerCase() !== incoming.outputToken.toLowerCase()) return false

	// Same-chain swaps ignore the declaration, as the orderbook does: there is no
	// source chain to accept or refuse.
	if (incoming.source === incoming.destination) return true
	return order.acceptedSources.includes(incoming.source)
}
