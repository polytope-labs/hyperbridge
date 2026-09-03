import { Mutex } from "async-mutex"
import { type DecodedOrderPlacedLog, type HexString, retryPromise } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { QuorumPublicClient } from "@/services/QuorumPublicClient"
import { DEFAULT_BLOCK_SCAN_INTERVAL_SECONDS } from "@/services/FillerConfigService"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { reconstructOrdersFromLogs } from "./reconstruct"
import type { ScannedFill, ScannedOrder } from "./types"

/** What one scan loop watches. Resolved by OrderScanner before construction. */
export interface ScanTarget {
	chain: string
	chainId: number
	gateway: HexString
	rpcUrls: string[]
}

/** Gateway events every scan subscribes to. */
const GATEWAY_EVENTS = INTENT_GATEWAY_V2_ABI.filter(
	(item) =>
		item.type === "event" &&
		(item.name === "OrderPlaced" || item.name === "OrderFilled" || item.name === "PartialFill"),
)

/**
 * Fallback when the scanner is built without one — the same default the config
 * service applies, so a scanner built directly scans at the rate a configured
 * one would. `blockScanIntervalSeconds` overrides it.
 */
const DEFAULT_SCAN_INTERVAL_MS = DEFAULT_BLOCK_SCAN_INTERVAL_SECONDS * 1000
const MAX_BLOCK_RANGE = 1_000n
/**
 * Events published in one synchronous stretch before yielding to the consumers'
 * drain microtasks. FanOut consumers hold 1000 events; a catch-up scan of a
 * busy range can reconstruct more than that, and a purely synchronous publish
 * loop would overflow the queues before any consumer got to run — dropping the
 * oldest orders with the cursor already past them, so no rescan ever sees them
 * again. A macrotask boundary flushes the entire microtask queue, i.e. lets
 * every consumer drain completely.
 */
const PUBLISH_CHUNK = 250
const yieldToConsumers = () => new Promise<void>((resolve) => setImmediate(resolve))

/** How long stop() waits for an in-flight scan before giving up on a clean drain. */
const STOP_DRAIN_TIMEOUT_MS = 5_000

/**
 * One block-scan loop for one (chain, gateway, endpoint set), feeding any number
 * of fillers.
 *
 * This is the loop that used to live inside every `EventMonitor`. Nothing about
 * the work is filler-specific — the `getLogs` filter carries no filler term, and
 * `OrderFilled.filler` is `indexed: false` so it could never be narrowed to a
 * topic anyway — which is why N fillers on one chain previously issued N
 * identical request streams forever.
 *
 * The cursor is shared, which changes one property worth naming: previously each
 * filler's cursor drifted independently, so a block one instance missed was
 * usually still seen by another. Here a missed range is missed by everyone, so
 * the loop never advances past a range it failed to read.
 */
export class ChainScanner {
	private quorumClient: QuorumPublicClient
	private readonly mutex = new Mutex()
	private orderHandler: (event: ScannedOrder) => void = () => {}
	private fillHandler: (event: ScannedFill) => void = () => {}
	private errorHandler: (error: unknown) => void = () => {}
	private readonly logger: Logger

	private timer?: NodeJS.Timeout
	private cursor: bigint | undefined
	private stopped = false

	constructor(
		private readonly target: ScanTarget,
		private readonly loggers: LoggerContext = defaultLoggerContext(),
		private readonly scanIntervalMs: number = DEFAULT_SCAN_INTERVAL_MS,
	) {
		this.logger = loggers.get("chain-scanner")
		this.quorumClient = new QuorumPublicClient(target.chainId, target.rpcUrls, loggers)
		if (this.quorumClient.size > 1) {
			this.logger.info(
				{
					chainId: target.chainId,
					providerCount: this.quorumClient.size,
					threshold: this.quorumClient.threshold,
				},
				"Quorum log scanning enabled",
			)
		}
	}

	get chainId(): number {
		return this.target.chainId
	}

	/** Blocks scanned so far. Undefined until the first head read lands. */
	get scannedTo(): bigint | undefined {
		return this.cursor
	}

	onOrder(handler: (event: ScannedOrder) => void): void {
		this.orderHandler = handler
	}

	onFill(handler: (event: ScannedFill) => void): void {
		this.fillHandler = handler
	}

	onError(handler: (error: unknown) => void): void {
		this.errorHandler = handler
	}

	/**
	 * Arms the scan loop. Idempotent, and does not wait on the network.
	 *
	 * There is deliberately no initial head read here: `scan()` already sets the
	 * cursor from the head on its first pass, so awaiting one would only make
	 * creating a scanner block on every chain's first round trip — and fail the
	 * whole scanner over an endpoint that is briefly slow, when the interval would
	 * have recovered on its own.
	 */
	start(): void {
		if (this.timer || this.stopped) return

		this.logger.info({ chainId: this.target.chainId }, "Block scanner started")
		this.timer = setInterval(() => {
			if (this.mutex.isLocked()) return
			void this.mutex.runExclusive(async () => {
				try {
					await this.scan()
				} catch (error) {
					// The cursor and endpoints are what an operator needs to act: the
					// error alone does not say which range failed or which provider set
					// produced it.
					this.logger.error(
						{
							chainId: this.target.chainId,
							cursor: this.cursor?.toString(),
							rpcUrls: this.target.rpcUrls,
							err: error,
						},
						"Error in block scanner",
					)
					this.errorHandler(error)
				}
			})
		}, this.scanIntervalMs)
	}

	/**
	 * Points this loop at different endpoints, keeping the cursor.
	 *
	 * Under the mutex, so the swap cannot land mid-scan and leave a pass that read
	 * its head from one provider set reading its logs from another. Keeping the
	 * cursor is the whole point: tearing the loop down and building a new one
	 * restarts from the head, silently skipping every block since the last scan.
	 */
	async setRpcUrls(rpcUrls: string[]): Promise<void> {
		await this.mutex.runExclusive(async () => {
			this.target.rpcUrls = rpcUrls
			this.quorumClient = new QuorumPublicClient(this.target.chainId, rpcUrls, this.loggers)
			this.logger.info(
				{ chainId: this.target.chainId, providerCount: this.quorumClient.size, cursor: this.cursor },
				"Swapped block scanner endpoints",
			)
		})
	}

	private async scan(): Promise<void> {
		const currentBlock = await retryPromise(() => this.quorumClient.getBlockNumber(), {
			maxRetries: 3,
			backoffMs: 250,
			logMessage: "Failed to get current block number",
		})

		// A stop() that timed out its drain has already resolved; whatever this scan
		// does now must be invisible. Checked after every await, because that is
		// where a stop can interleave.
		if (this.stopped) return

		// `undefined`, not falsy: the previous implementation tested `if (!lastScanned)`,
		// which is also true for 0n — so a chain whose head was 1 at boot never scanned.
		if (this.cursor === undefined) {
			this.cursor = currentBlock - 1n
			return
		}
		if (currentBlock <= this.cursor) return

		const fromBlock = this.cursor + 1n
		const toBlock = fromBlock + MAX_BLOCK_RANGE > currentBlock ? currentBlock : fromBlock + MAX_BLOCK_RANGE

		this.logger.debug(
			{ chainId: this.target.chainId, fromBlock, toBlock, gap: Number(toBlock - fromBlock) },
			"Scanning blocks",
		)

		let logs: Array<Record<string, unknown>>
		try {
			logs = await retryPromise(
				() =>
					this.quorumClient.getLogs({
						address: this.target.gateway,
						events: GATEWAY_EVENTS,
						fromBlock,
						toBlock,
					}),
				{ maxRetries: 3, backoffMs: 250, logMessage: "Failed to get gateway event logs" },
			)
		} catch (error) {
			// The RPC has not indexed these blocks yet. Do not advance the cursor —
			// with one shared cursor, skipping a range loses it for every consumer.
			if (isBlockRangeError(error)) return
			throw error
		}

		if (this.stopped) return

		const placed = logs.filter((log) => log.eventName === "OrderPlaced")
		const filled = logs.filter((log) => log.eventName === "OrderFilled" || log.eventName === "PartialFill")

		if (placed.length > 0) {
			this.logger.info(
				{ chainId: this.target.chainId, fromBlock, toBlock, eventCount: placed.length },
				"Found OrderPlaced events in block scan",
			)
			await this.publishOrders(placed as unknown as DecodedOrderPlacedLog[])
		}

		if (filled.length > 0) {
			this.logger.info(
				{ chainId: this.target.chainId, fromBlock, toBlock, eventCount: filled.length },
				"Found OrderFilled events in block scan",
			)
			await this.publishFills(filled)
		}

		// The publishes above yield, so stop() may have landed inside them. An
		// aborted publish delivered only part of the range; advancing the cursor
		// would mark the rest scanned when nobody received it. Leaving it means a
		// restart rescans the range — at-least-once, which subscribers already
		// tolerate.
		if (this.stopped) return
		this.cursor = toBlock
	}

	private async publishOrders(logs: DecodedOrderPlacedLog[]): Promise<void> {
		const rebuilt = reconstructOrdersFromLogs(logs, {
			onError: (err, log) => this.logger.error({ err, log }, "Error parsing event log"),
		})

		let published = 0
		for (const entry of rebuilt) {
			this.logger.info({ orderId: entry.order.id, txHash: entry.transactionHash }, "New order detected")
			this.orderHandler({ ...entry, chain: this.target.chain, chainId: this.target.chainId })
			if (++published % PUBLISH_CHUNK === 0) {
				await yieldToConsumers()
				// A yield is an await, and awaits are where stop() interleaves. Aborting
				// here also releases the scan mutex within one chunk, so stop()'s drain
				// finishes in milliseconds instead of racing its timeout.
				if (this.stopped) return
			}
		}
	}

	private async publishFills(logs: Array<Record<string, unknown>>): Promise<void> {
		let published = 0
		for (const log of logs) {
			if (++published % PUBLISH_CHUNK === 0) {
				await yieldToConsumers()
				if (this.stopped) return
			}
			try {
				const args = log.args as { commitment?: HexString; filler?: string } | undefined
				const commitment = args?.commitment
				if (!commitment) {
					this.logger.warn({ log }, "OrderFilled log missing commitment")
					continue
				}
				const coords = log as unknown as LogCoords
				this.fillHandler({
					commitment,
					filler: args?.filler ?? "",
					chainId: this.target.chainId,
					blockNumber: coords.blockNumber ?? 0n,
					blockHash: coords.blockHash ?? "",
					logIndex: coords.logIndex ?? 0,
				})
			} catch (error) {
				this.logger.error({ err: error, log }, "Error parsing OrderFilled log")
			}
		}
	}

	/** Stops the loop. Called by the registry when the last consumer releases. */
	async stop(): Promise<void> {
		this.stopped = true
		if (this.timer) {
			clearInterval(this.timer)
			this.timer = undefined
		}
		// Wait out an in-flight scan so nothing is still retrying or publishing after
		// this resolves — otherwise a host that stopped every filler still refuses to
		// exit. Bounded, though: a scan blocked on an unresponsive endpoint sits
		// inside a retry ladder that can run for minutes, and shutdown must not.
		// `stopped` is already set, so a scan that outlives the wait publishes
		// nothing and its interval is cleared.
		await Promise.race([
			this.mutex.runExclusive(async () => {}),
			new Promise<void>((resolve) => setTimeout(resolve, STOP_DRAIN_TIMEOUT_MS).unref?.()),
		])
		this.logger.info({ chainId: this.target.chainId }, "Shared block scanner stopped")
	}
}

interface LogCoords {
	blockNumber?: bigint
	blockHash?: string
	logIndex?: number
}

function isBlockRangeError(error: unknown): boolean {
	const err = error as { message?: string; details?: string } | undefined
	const message = String(err?.message || err?.details || "")
	return (
		message.includes("block range extends beyond current head block") ||
		message.includes("invalid block range params")
	)
}
