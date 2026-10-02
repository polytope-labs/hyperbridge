import { useEffect, useState } from "react"
import { api } from "../../api"
import type { OrderbookLevelOrder } from "../../types"
import type { DepthLevel } from "./orderbookModel"

interface LevelOrdersState {
	key: string
	orders?: OrderbookLevelOrder[]
	error?: string
}

/** The orders behind one level, read once when it is opened. Nothing is read while `level` is null. */
export function useLevelOrders(book: string, level: DepthLevel | null, sourceChain: string) {
	const params = new URLSearchParams()
	if (level) {
		params.set("book", book)
		params.set("side", level.side)
		params.set("fillChain", level.fillChain)
		params.set("priceBucket", level.priceBucket)
		if (sourceChain) params.set("sourceChain", sourceChain)
	}
	const key = level ? params.toString() : ""
	const [state, setState] = useState<LevelOrdersState>({ key: "" })

	useEffect(() => {
		if (!key) return
		const controller = new AbortController()
		api.get<OrderbookLevelOrder[]>(`/api/orderbook/level-orders?${key}`, { signal: controller.signal })
			.then((orders) => {
				if (!controller.signal.aborted) setState({ key, orders })
			})
			.catch((err: unknown) => {
				if (!controller.signal.aborted)
					setState({ key, error: err instanceof Error ? err.message : "Could not read this level's orders" })
			})
		return () => controller.abort()
	}, [key])

	return state.key === key ? state : { key }
}
