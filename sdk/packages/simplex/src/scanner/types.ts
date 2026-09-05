import type { HexString, Order, PhantomOrderEvent } from "@hyperbridge/sdk"
import type { LoggerContext } from "@/services/Logger"

/**
 * Shared event scanners.
 *
 * Scanning a chain is identical work for every filler: the `getLogs` filter is
 * `{ address: gateway, events: [OrderPlaced, OrderFilled, PartialFill] }` with
 * no filler-specific term, and rebuilding an `Order` from a log is a pure
 * function needing no further RPC. So N fillers on a chain issue N copies of the
 * same request stream unless they share one.
 *
 * Sharing is something you do on purpose: build a scanner, hand it to the fillers
 * that should share it, and close it when you are done. A filler started without
 * one builds its own private scanner and closes it on `stop()`, so nothing is
 * ever shared behind your back.
 */

/** A chain to scan. The same shape as a `chains` entry in the filler config. */
export interface ScannerChainConfig {
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
	/** Attribution tag the placer supplied; absent on pre-#1092 logs and on feeds that lack it. */
	graffiti?: HexString
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
	/** Hash of the transaction that filled the order, when the log carried it. */
	transactionHash?: string
}

/** What {@link OrderScanner.create} needs to start scanning. */
export interface OrderScannerOptions {
	/** Chains to scan. The same shape as the filler config's `chains` array. */
	chains: ScannerChainConfig[]
	/**
	 * Seconds between scans of each chain. Defaults to 3, the same default
	 * `blockScanIntervalSeconds` carries. Minimum 0.1.
	 */
	scanIntervalSecs?: number
	/** Where this scanner logs. Defaults to the process-wide fallback context. */
	loggers?: LoggerContext
}

export interface OrderScannerHandlers {
	onOrder(event: ScannedOrder): void
	onFill(event: ScannedFill): void
	/**
	 * A scan failed. Informational — the scanner retries on its own schedule and
	 * never advances its cursor past a range it could not read.
	 */
	onError?(error: unknown, chainId: number): void
}

export interface Subscription {
	/** Detaches this subscriber. Does not stop the scanner. */
	close(): void
	/**
	 * Events dropped because this subscriber could not keep up. Non-zero means
	 * orders were missed: a scanner never blocks on a slow subscriber, because one
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
export interface OrderScanner {
	/** Receives events for every chain in the scanner; subscribers match on `chainId`. */
	subscribe(handlers: OrderScannerHandlers): Subscription
	/** Chain ids currently scanned. */
	chains(): number[]
	/** Begins scanning another chain. */
	addChain(chain: ScannerChainConfig): Promise<number>
	/** Points a chain at different endpoints, keeping its cursor so no block is skipped. */
	setRpcUrls(chainId: number, rpcUrls: string[]): Promise<void>
	/** Stops scanning a chain. Subscribers simply stop seeing it. */
	removeChain(chainId: number): Promise<void>
	/** Stops every scan loop. The scanner cannot be reused afterwards. */
	close(): Promise<void>
}

export interface HyperbridgeScannerHandlers {
	/**
	 * One call per Hyperbridge block, carrying every phantom order registered in
	 * it. The batch boundary is load-bearing: the pallet registers one order per
	 * configured chain in the same block, and a solver bids on the whole set in
	 * a single extrinsic — per-order delivery would put every chain's bid behind
	 * the previous one's inclusion on the account nonce.
	 */
	onPhantomOrders(orders: PhantomOrderEvent[]): void
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
export interface HyperbridgeScanner {
	subscribe(handlers: HyperbridgeScannerHandlers): Subscription
	close(): Promise<void>
}
