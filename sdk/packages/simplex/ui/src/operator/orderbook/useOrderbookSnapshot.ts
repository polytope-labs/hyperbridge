import { useEffect, useState } from "react"
import { api } from "../../api"
import type { OrderbookSnapshot } from "../../types"

interface SnapshotState {
	key: string
	snapshot?: OrderbookSnapshot
	error?: string
	updatedAt?: number
}

/** One request at a time. A changed route aborts its old request and hides old prices. */
export function useOrderbookSnapshot(book: string, sourceChain: string, fillChain: string) {
	const params = new URLSearchParams({ book })
	if (sourceChain) params.set("sourceChain", sourceChain)
	if (fillChain) params.set("fillChain", fillChain)
	const key = params.toString()
	const [state, setState] = useState<SnapshotState>({ key: "" })

	useEffect(() => {
		if (!book) return
		const controller = new AbortController()
		let timer: ReturnType<typeof setTimeout>
		const load = async () => {
			try {
				const snapshot = await api.get<OrderbookSnapshot>(`/api/orderbook/snapshot?${key}`, {
					signal: controller.signal,
				})
				if (!controller.signal.aborted) setState({ key, snapshot, updatedAt: Date.now() })
			} catch (err) {
				if (!controller.signal.aborted)
					setState((previous) => ({
						...(previous.key === key ? previous : { key }),
						error: err instanceof Error ? err.message : "Could not read the order book",
					}))
			} finally {
				if (!controller.signal.aborted) timer = setTimeout(() => void load(), 10_000)
			}
		}
		void load()
		return () => {
			controller.abort()
			clearTimeout(timer)
		}
	}, [book, key])

	return state.key === key ? state : { key }
}
