import { MemoryDataStore } from "@/data/memory"
import type { LimitOrderSide, LimitOrderStore } from "@/data/types"
import { ORDERBOOK_SCALE } from "@/orderbook/amounts"

/** One resting order, written the way an operator would state it. */
export interface TestLimitOrder {
	base: string
	quote: string
	/** BID takes the base in and pays the quote out; ASK is the other way round. */
	side: LimitOrderSide
	fillChain: string
	/** Quote per 1 base, in whole units (e.g. "1500"). */
	price: string
	/** What the order will pay out in total, in whole units of the token it pays. */
	size: string
	/** Defaults to accepting the fill chain itself, which covers same-chain tests. */
	acceptedSources?: string[]
	id?: string
}

/** Whole units to the 1e18 the store and matcher work in. */
function scale(amount: string): string {
	const [whole, fraction = ""] = amount.split(".")
	const padded = (fraction + "0".repeat(18)).slice(0, 18)
	return (BigInt(whole) * ORDERBOOK_SCALE + BigInt(padded || "0")).toString()
}

/**
 * A limit order store holding the orders a test prices against.
 *
 * The filler has no prices of its own, so a test that expects a fill has to say
 * what the operator was offering, the same way a live filler would only fill
 * what a resting order covers.
 */
export async function limitOrderStore(orders: TestLimitOrder[]): Promise<LimitOrderStore> {
	const store = new MemoryDataStore().limitOrders
	for (const [index, order] of orders.entries()) {
		await store.create({
			id: order.id ?? `limit-${index}`,
			book: `${order.base}/${order.quote}`,
			base: order.base,
			quote: order.quote,
			side: order.side,
			fillChain: order.fillChain,
			price: scale(order.price),
			size: scale(order.size),
			acceptedSources: order.acceptedSources ?? [order.fillChain],
			ttlSecs: 900,
		})
	}
	return store
}
