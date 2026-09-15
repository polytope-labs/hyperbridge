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
	const available = BigInt(order.remaining) - BigInt(order.reserved)
	return available > 0n ? available : 0n
}

/**
 * The limit order an incoming order is priced against, or null when none serves it.
 *
 * There is no fallback price: an order matching nothing is not filled, which is
 * the whole point of pricing from the operator's own resting orders rather than
 * from a curve that always has an answer.
 *
 * When several match, the largest offer wins, and a tie goes to the one with
 * more left. That is the order the orderbook would have served when the swapper
 * was quoted. Exactly one limit order is ever returned, which is what keeps the
 * draw-down on a fill a one-to-one piece of bookkeeping.
 */
export function matchLimitOrder(
	orders: readonly LimitOrder[],
	incoming: IncomingOrder,
	resolve: (symbol: string, chain: string) => HexString | null,
	now: Date = new Date(),
): LimitOrderMatch | null {
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
		.filter((candidate) => candidate.offer >= incoming.requestedOutput)

	return candidates.reduce<LimitOrderMatch | null>((best, candidate) => {
		if (!best) return candidate
		if (candidate.offer !== best.offer) return candidate.offer > best.offer ? candidate : best
		return candidate.available > best.available ? candidate : best
	}, null)
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
