import type { HyperbridgeStream, OrderStream, OrderStreamHandlers } from "@/scanner/types"

/**
 * An order stream that scans nothing.
 *
 * `IntentFiller` requires one, but plenty of tests exercise paths that never
 * involve a scan — retraction, phantom gating, strategy pricing. `emit` lets a
 * test push an event through as if a chain had produced it.
 */
export function stubOrderStream(chains: number[] = []): OrderStream & {
	emit: OrderStreamHandlers
} {
	let handlers: OrderStreamHandlers | undefined
	return {
		subscribe: (h) => {
			handlers = h
			return { close: () => (handlers = undefined), dropped: 0 }
		},
		chains: () => [...chains],
		addChain: async () => 0,
		removeChain: async () => {},
		close: async () => {},
		emit: {
			onOrder: (event) => handlers?.onOrder(event),
			onFill: (event) => handlers?.onFill(event),
			onError: (error, chainId) => handlers?.onError?.(error, chainId),
		},
	}
}

export function stubHyperbridgeStream(): HyperbridgeStream {
	return {
		subscribe: () => ({ close: () => {}, dropped: 0 }),
		close: async () => {},
	}
}
