import { useCallback, useEffect, useRef, useState } from "react"
import { api, ApiError } from "../../api"
import type { CreateLimitOrderRequest, LimitOrder, LimitOrderFill, LimitOrderStatus, StoredBid } from "../../types"

/** How often the list refreshes itself: a posting lands, expires or is filled without the operator acting. */
const POLL_MS = 10_000

export interface LimitOrderFills {
	order: LimitOrder
	/** Every fill that drew the order down, kept across its resizes. */
	fills: LimitOrderFill[]
	/** The bids that drew on it and have not settled yet. */
	bids: StoredBid[]
}

/**
 * The operator's limit orders, kept current while the page is open.
 *
 * Every mutation reloads rather than patching in place: creating an order posts
 * it to the orderbook, and what comes back — a commitment, a refusal on the row
 * — is the server's answer rather than anything this can predict.
 */
export function useLimitOrders(options: { status?: LimitOrderStatus | "" } = {}) {
	const { status = "" } = options
	const [orders, setOrders] = useState<LimitOrder[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string>()
	// A reload that lands after the component is gone must not set state.
	const live = useRef(true)

	const load = useCallback(async () => {
		try {
			const query = status ? `?status=${encodeURIComponent(status)}` : ""
			const body = await api.get<{ orders: LimitOrder[] }>(`/api/limit-orders${query}`)
			if (!live.current) return
			setOrders(body.orders)
			setError(undefined)
		} catch (err) {
			if (!live.current) return
			setError(err instanceof ApiError ? err.message : "Could not read the limit orders")
		} finally {
			if (live.current) setLoading(false)
		}
	}, [status])

	useEffect(() => {
		live.current = true
		void load()
		const timer = setInterval(() => void load(), POLL_MS)
		return () => {
			live.current = false
			clearInterval(timer)
		}
	}, [load])

	const create = useCallback(
		async (request: CreateLimitOrderRequest) => {
			await api.post("/api/limit-orders", request)
			await load()
		},
		[load],
	)

	const cancel = useCallback(
		async (id: string) => {
			await api.del(`/api/limit-orders/${id}`)
			await load()
		},
		[load],
	)

	const withFills = useCallback((id: string) => api.get<LimitOrderFills>(`/api/limit-orders/${id}`), [])

	return { orders, loading, error, reload: load, create, cancel, withFills }
}
