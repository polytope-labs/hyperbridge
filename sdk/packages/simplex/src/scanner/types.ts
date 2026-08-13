import type { HexString, Order, PhantomOrderEvent } from "@hyperbridge/sdk"

/**
 * Shared event sources.
 *
 * Scanning a chain for gateway events is identical work for every filler: the
 * `getLogs` filter is `{ address: gateway, events: [OrderPlaced, OrderFilled,
 * PartialFill] }` with no filler-specific term, and rebuilding an `Order` from a
 * log is a pure function that needs no further RPC. N fillers pointed at the same
 * chain therefore issue N copies of the same request forever. The same holds on
 * the Hyperbridge side, where every instance re-reads every block for phantom
 * orders.
 *
 * These interfaces let one scan loop feed many consumers. The default
 * implementations are process-local and refcounted, so several `Simplex`
 * instances in one process share automatically; a host that runs fillers across
 * processes can implement them over its own transport.
 */

/** A chain a consumer wants events for. */
export interface ScanTarget {
	/** State machine id, e.g. "EVM-8453". */
	chain: string
	chainId: number
	/** IntentGateway address on this chain. */
	gateway: HexString
	/** Endpoints to scan with. Order is not significant; the scan key sorts them. */
	rpcUrls: string[]
}

/**
 * Identity of a scan loop. Two consumers share a loop only when all three match.
 *
 * The endpoints are part of the key, not just the chain: a quorum's claim is
 * "two thirds of *these* endpoints agree", so folding two consumers with
 * different endpoint sets onto one loop would silently give one of them a
 * consensus it never asked for. The gateway address is in the key because a
 * redeployed gateway is a different event stream on the same chain.
 */
export function scanKey(target: ScanTarget): string {
	return `${target.chainId}:${target.gateway.toLowerCase()}:${[...target.rpcUrls].sort().join(",")}`
}

/** An order observed on chain, with the log coordinates a cursor needs. */
export interface ScannedOrder {
	order: Order
	transactionHash: string
	blockNumber: bigint
	blockHash: string
	logIndex: number
	/** State machine id of the chain it was observed on. */
	chain: string
	chainId: number
}

/** A fill observed on chain. Not filtered by filler — consumers do that themselves. */
export interface ScannedFill {
	commitment: HexString
	/** The address credited with the fill. `filler` is `indexed: false` in the ABI, so this
	 *  can never be narrowed to a topic filter — every consumer receives every fill. */
	filler: string
	chainId: number
	blockNumber: bigint
	blockHash: string
	logIndex: number
}

export interface OrderSourceHandlers {
	onOrder(event: ScannedOrder): void
	onFill(event: ScannedFill): void
	/**
	 * Called when the loop feeding this subscription fails a scan. Informational —
	 * the loop retries on its own schedule and the cursor does not advance past an
	 * unread range.
	 */
	onError?(error: unknown, chainId: number): void
}

/** Live subscription to one chain's events. */
export interface Subscription {
	/**
	 * Detaches this consumer. The underlying loop keeps running while other
	 * consumers hold it, and stops when the last one releases.
	 */
	close(): void
	/**
	 * Events dropped for this consumer because it could not keep up. Non-zero
	 * means orders were missed — the fan-out never blocks the shared producer, so
	 * a slow consumer loses events rather than stalling the fleet.
	 */
	readonly dropped: number
}

/**
 * A source of on-chain gateway events.
 *
 * Implementations must deliver per-chain FIFO. Cross-chain ordering has never
 * existed (each chain has always had its own interval and cursor) and nothing
 * depends on it.
 *
 * Delivery is at-least-once: a source that replays from a cursor after a
 * reconnect will re-deliver. Consumers de-duplicate on `order.id` — which is why
 * `EventMonitor` keeps a seen-set rather than relying, as it used to, on a
 * private monotonic cursor making duplicates impossible by construction.
 */
export interface OrderSource {
	subscribe(target: ScanTarget, handlers: OrderSourceHandlers): Subscription
	/** Chains this source is currently scanning, for health reporting. */
	activeChains(): number[]
}

// ===========================================================================
// Hyperbridge
// ===========================================================================

export interface HyperbridgeHandlers {
	onPhantomOrder(event: PhantomOrderEvent): void
	onError?(error: unknown): void
}

/**
 * A source of Hyperbridge-side events.
 *
 * Phantom order registration is chain-global, exactly like `OrderPlaced`, so the
 * per-block read every instance performs is duplicated work. Sharing it matters
 * more than sharing the EVM scan: phantom polling reads every Hyperbridge block
 * through `offchain_localStorageGet`, which requires `--rpc-methods=unsafe`, so
 * it can only ever hit an operator's own node.
 *
 * Bid submission is *not* here — it is signed with an instance's own substrate
 * key and stays on that instance's `IntentsCoprocessor`.
 */
export interface HyperbridgeSource {
	subscribe(wsUrl: string, handlers: HyperbridgeHandlers): Subscription
}
