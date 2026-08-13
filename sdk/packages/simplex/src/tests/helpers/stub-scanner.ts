import type { HyperbridgeScanner, OrderScanner, OrderScannerHandlers } from "@/scanner/types"

/**
 * An order scanner that scans nothing.
 *
 * `IntentFiller` requires one, but plenty of tests exercise paths that never
 * involve a scan — retraction, phantom gating, strategy pricing. `emit` lets a
 * test push an event through as if a chain had produced it.
 */
export function stubOrderScanner(chains: number[] = []): OrderScanner & {
	emit: OrderScannerHandlers
} {
	let handlers: OrderScannerHandlers | undefined
	return {
		subscribe: (h) => {
			handlers = h
			return { close: () => (handlers = undefined), dropped: 0 }
		},
		chains: () => [...chains],
		addChain: async () => 0,
		setRpcUrls: async () => {},
		removeChain: async () => {},
		close: async () => {},
		emit: {
			onOrder: (event) => handlers?.onOrder(event),
			onFill: (event) => handlers?.onFill(event),
			onError: (error, chainId) => handlers?.onError?.(error, chainId),
		},
	}
}

export function stubHyperbridgeScanner(): HyperbridgeScanner {
	return {
		subscribe: () => ({ close: () => {}, dropped: 0 }),
		close: async () => {},
	}
}
