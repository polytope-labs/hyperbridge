import type { HexString, Order, PhantomOrderEvent } from "@hyperbridge/sdk"

/**
 * Shared event streams.
 *
 * Scanning a chain is identical work for every filler: the `getLogs` filter is
 * `{ address: gateway, events: [OrderPlaced, OrderFilled, PartialFill] }` with
 * no filler-specific term, and rebuilding an `Order` from a log is a pure
 * function needing no further RPC. So N fillers on a chain issue N copies of the
 * same request stream unless they share one.
 *
 * Sharing is something you do on purpose: build a stream, hand it to the fillers
 * that should share it, and close it when you are done. A filler started without
 * one builds its own private stream and closes it on `stop()`, so nothing is
 * ever shared behind your back.
 */

/** A chain to scan. The same shape as a `chains` entry in the filler config. */
export interface StreamChainConfig {
	/** One or more endpoints. Several enables quorum log scanning. */
	rpcUrls: string[]
	/** Carried so a config's `chains` array can be passed through unchanged; unused for scanning. */
	bundlerUrl?: string
	/**
	 * Chain id, when you already know it. Omit and it is read back from the
	 * endpoints at `create` time, which also proves they answer for one chain.
	 */
	chainId?: number
	/** IntentGateway address. Omit to take it from the SDK chain registry. */
	gateway?: HexString
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

/** A fill observed on chain. */
export interface ScannedFill {
	commitment: HexString
	/**
	 * The address credited with the fill. `filler` is `indexed: false` on both
	 * `OrderFilled` and `PartialFill`, so it can never be narrowed to a topic
	 * filter — every subscriber receives every fill and matches its own address.
	 */
	filler: string
	chainId: number
	blockNumber: bigint
	blockHash: string
	logIndex: number
}

export interface OrderStreamHandlers {
	onOrder(event: ScannedOrder): void
	onFill(event: ScannedFill): void
	/**
	 * A scan failed. Informational — the stream retries on its own schedule and
	 * never advances its cursor past a range it could not read.
	 */
	onError?(error: unknown, chainId: number): void
}

export interface Subscription {
	/** Detaches this subscriber. Does not stop the stream. */
	close(): void
	/**
	 * Events dropped because this subscriber could not keep up. Non-zero means
	 * orders were missed: a stream never blocks on a slow subscriber, because one
	 * would otherwise stall every other filler reading from it.
	 */
	readonly dropped: number
}

/**
 * A live feed of gateway events for a fixed set of chains.
 *
 * Delivery is at-least-once and per-chain FIFO. Subscribers de-duplicate on
 * `order.id` — `EventMonitor` does, and a custom subscriber must too.
 */
export interface OrderStream {
	/** Receives events for every chain in the stream; subscribers match on `chainId`. */
	subscribe(handlers: OrderStreamHandlers): Subscription
	/** Chain ids currently scanned. */
	chains(): number[]
	/** Begins scanning another chain. */
	addChain(chain: StreamChainConfig): Promise<number>
	/** Stops scanning a chain. Subscribers simply stop seeing it. */
	removeChain(chainId: number): Promise<void>
	/** Stops every scan loop. The stream cannot be reused afterwards. */
	close(): Promise<void>
}

export interface HyperbridgeStreamHandlers {
	onPhantomOrder(event: PhantomOrderEvent): void
	onError?(error: unknown): void
}

/**
 * A live feed of Hyperbridge phantom orders.
 *
 * The heavier of the two to share: phantom polling re-reads every Hyperbridge
 * block, and it goes through `offchain_localStorageGet`, which needs
 * `--rpc-methods=unsafe` — so it can only ever hit an operator's own node.
 *
 * Reads only. Bids are signed with a filler's own substrate key on its own
 * connection and never come through here.
 */
export interface HyperbridgeStream {
	subscribe(handlers: HyperbridgeStreamHandlers): Subscription
	close(): Promise<void>
}
