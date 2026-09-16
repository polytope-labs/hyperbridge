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
 * The limit orders an incoming order is priced against, in the order they should
 * be drawn on, or an empty list when none serves it.
 *
 * There is no fallback price: an order matching nothing is not filled, which is
 * the whole point of pricing from the operator's own resting orders rather than
 * from a curve that always has an answer.
 *
 * An order takes part only if its offer covers what the swapper asked for. That
 * is not a preference, it is what the operator's rate permits: escrow release is
 * strictly proportional, `Released(filled) = escrowTotal * filled / totalRequired`,
 * so a fill of `f` out of `T` releases `I * f / T` and settles at `T / I`
 * whatever `f` is. Every fill of an order therefore pays the swapper's rate, and
 * an order may only take part where that rate is inside its own: `T / I <= price`,
 * which is exactly `offer >= requestedOutput`.
 *
 * Several may be needed, because one that clears the rate may not have the depth.
 * The orderbook already quotes a swapper across every level that can fill the
 * trade together, so serving only the first would advertise depth and refuse to
 * meet it. What is paid in total is the ask, never the sum of the offers: each
 * order funds a slice, is drawn down by that slice, and receives that fraction of
 * the input, so every one of them settles at `T / I` too.
 *
 * They are drawn on tightest first, meaning the smallest offer that still clears
 * the ask. Because the rate is the swapper's either way, which order funds a
 * slice does not change what this swap earns; it decides what is left afterwards.
 * A more generous order qualifies for every swap a tighter one does and for swaps
 * it cannot serve, at the same cost per unit, so the tight end is what to spend.
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
		.sort(byTightestFirst)

	const taken: LimitOrderMatch[] = []
	let covered = 0n
	for (const candidate of qualifying) {
		taken.push(candidate)
		covered += candidate.payout
		if (covered >= incoming.requestedOutput) break
	}
	return taken
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
 * Smallest offer first, then least left, then by id.
 *
 * Spending the tight end keeps the generous orders resting, and finishing the
 * smaller of two equal offers retires it rather than leaving two part-used. The
 * last comparison is what makes the sequence reproducible, because the draw-down
 * after a fill walks these orders in the same order.
 */
function byTightestFirst(a: LimitOrderMatch, b: LimitOrderMatch): number {
	if (a.offer !== b.offer) return a.offer < b.offer ? -1 : 1
	if (a.available !== b.available) return a.available < b.available ? -1 : 1
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
