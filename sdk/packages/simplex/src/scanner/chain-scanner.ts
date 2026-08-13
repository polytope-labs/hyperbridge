import { Mutex } from "async-mutex"
import { type DecodedOrderPlacedLog, type HexString, retryPromise } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { QuorumPublicClient } from "@/services/QuorumPublicClient"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { reconstructOrdersFromLogs } from "./reconstruct"
import { FanOut } from "./fan-out"
import type { OrderSourceHandlers, ScanTarget, ScannedFill, ScannedOrder, Subscription } from "./types"

/** Gateway events every scan subscribes to. */
const GATEWAY_EVENTS = INTENT_GATEWAY_V2_ABI.filter(
	(item) =>
		item.type === "event" &&
		(item.name === "OrderPlaced" || item.name === "OrderFilled" || item.name === "PartialFill"),
)

const SCAN_INTERVAL_MS = 1_000
const MAX_BLOCK_RANGE = 1_000n

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
	private readonly quorumClient: QuorumPublicClient
	private readonly orders = new FanOut<ScannedOrder>("orders")
	private readonly fills = new FanOut<ScannedFill>("fills")
	private readonly errors = new Set<(error: unknown, chainId: number) => void>()
	private readonly mutex = new Mutex()
	private readonly logger: Logger

	private timer?: NodeJS.Timeout
	private cursor: bigint | undefined
	private stopped = false
	private starting?: Promise<void>

	constructor(
		private readonly target: ScanTarget,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("chain-scanner")
		this.quorumClient = new QuorumPublicClient(target.chainId, target.rpcUrls)
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

	/** Attaches a consumer, starting the loop if this is the first one. */
	subscribe(handlers: OrderSourceHandlers): Subscription {
		const orderConsumer = this.orders.add(handlers.onOrder)
		const fillConsumer = this.fills.add(handlers.onFill)
		if (handlers.onError) this.errors.add(handlers.onError)

		void this.start()

		return {
			close: () => {
				orderConsumer.close()
				fillConsumer.close()
				if (handlers.onError) this.errors.delete(handlers.onError)
			},
			get dropped() {
				return orderConsumer.dropped + fillConsumer.dropped
			},
		}
	}

	/** Idempotent. Called on every subscribe; only the first one arms the timer. */
	async start(): Promise<void> {
		if (this.timer || this.stopped) return
		// Concurrent subscribers await the same in-flight start rather than racing to
		// arm two intervals, and stop() has something to wait on.
		if (this.starting) return this.starting
		this.starting = this.begin()
		return this.starting
	}

	private async begin(): Promise<void> {
		try {
			const head = await retryPromise(() => this.quorumClient.getBlockNumber(), {
				maxRetries: 3,
				backoffMs: 250,
				logMessage: "Failed to get start block number",
			})
			// Scan from the head itself, not past it.
			this.cursor = head - 1n
		} catch (error) {
			// Leave the cursor unset and let the interval retry; a scanner that
			// cannot read the head yet must not silently report itself healthy.
			this.logger.error({ chainId: this.target.chainId, err: error }, "Failed to read start block")
		}

		// The head read above can take a while against a slow or unreachable endpoint,
		// and the last consumer may have released in the meantime. Arming the interval
		// now would leave a loop nobody is listening to holding the process open.
		if (this.stopped) return

		this.logger.info({ chainId: this.target.chainId, startBlock: this.cursor }, "Shared block scanner started")

		this.timer = setInterval(() => {
			if (this.mutex.isLocked()) return
			void this.mutex.runExclusive(async () => {
				try {
					await this.scan()
				} catch (error) {
					this.logger.error({ chainId: this.target.chainId, err: error }, "Error in block scanner")
					for (const onError of this.errors) {
						try {
							onError(error, this.target.chainId)
						} catch {
							// A consumer's error handler must not break the loop for everyone else.
						}
					}
				}
			})
		}, SCAN_INTERVAL_MS)
	}

	private async scan(): Promise<void> {
		const currentBlock = await retryPromise(() => this.quorumClient.getBlockNumber(), {
			maxRetries: 3,
			backoffMs: 250,
			logMessage: "Failed to get current block number",
		})

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

		const placed = logs.filter((log) => log.eventName === "OrderPlaced")
		const filled = logs.filter((log) => log.eventName === "OrderFilled" || log.eventName === "PartialFill")

		if (placed.length > 0) {
			this.logger.info(
				{ chainId: this.target.chainId, fromBlock, toBlock, eventCount: placed.length },
				"Found OrderPlaced events in block scan",
			)
			this.publishOrders(placed as unknown as DecodedOrderPlacedLog[])
		}

		if (filled.length > 0) {
			this.logger.info(
				{ chainId: this.target.chainId, fromBlock, toBlock, eventCount: filled.length },
				"Found OrderFilled events in block scan",
			)
			this.publishFills(filled)
		}

		this.cursor = toBlock
	}

	private publishOrders(logs: DecodedOrderPlacedLog[]): void {
		const rebuilt = reconstructOrdersFromLogs(logs, {
			onError: (err, log) => this.logger.error({ err, log }, "Error parsing event log"),
		})

		for (const entry of rebuilt) {
			this.logger.info({ orderId: entry.order.id, txHash: entry.transactionHash }, "New order detected")
			this.orders.publish({ ...entry, chain: this.target.chain, chainId: this.target.chainId })
		}
	}

	private publishFills(logs: Array<Record<string, unknown>>): void {
		for (const log of logs) {
			try {
				const args = log.args as { commitment?: HexString; filler?: string } | undefined
				const commitment = args?.commitment
				if (!commitment) {
					this.logger.warn({ log }, "OrderFilled log missing commitment")
					continue
				}
				const coords = log as unknown as LogCoords
				this.fills.publish({
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
		// Wait out both an in-flight start and an in-flight scan, so nothing is still
		// retrying or publishing after this resolves — otherwise a host process that
		// stopped every filler still refuses to exit.
		await this.starting?.catch(() => {})
		await this.mutex.runExclusive(async () => {})
		if (this.timer) {
			clearInterval(this.timer)
			this.timer = undefined
		}
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
