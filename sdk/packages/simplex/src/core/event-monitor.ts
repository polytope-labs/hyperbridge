import { EventEmitter } from "events"
import {
	ChainConfig,
	Order,
	orderCommitment,
	normalizeStateMachineId,
	retryPromise,
	DecodedOrderPlacedLog,
	HexString,
} from "@hyperbridge/sdk"
import { ChainClientManager } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { type Logger, moduleLogger } from "@/services/Logger"
import { SharedOrderSource } from "@/scanner/registry"
import { reconstructOrdersFromLogs, type ReconstructDeps, type ReconstructedOrder } from "@/scanner/reconstruct"
import type { OrderSource, ScanTarget, Subscription } from "@/scanner/types"

// Re-exported from its original home so existing importers keep working.
export { reconstructOrdersFromLogs }
export type { ReconstructDeps, ReconstructedOrder }

/** Order ids retained for de-duplication. Roughly an hour of mainnet flow. */
const SEEN_LIMIT = 5_000

/**
 * Per-instance event bus for one filler.
 *
 * This used to own a block-scan loop per chain. It no longer does: scanning is
 * identical work for every filler on a chain, so it moved to a shared
 * {@link OrderSource} and this class subscribes to it. Everything else stays —
 * the monitor is the filler's event bus, not merely a source. `IntentFiller`
 * emits `orderTiming`, `orderSkipped`, `orderFilled`, `orderExecuted` and
 * `rebalanceExecuted` on it, `ActivityRecorder` attaches to it, and `Simplex`
 * re-exposes it as the public event surface.
 *
 * Two things must stay on this side of the boundary:
 *  - the `OrderFilled` filler-address filter. `filler` is `indexed: false` on
 *    both `OrderFilled` and `PartialFill`, so it can never be a topic filter;
 *    every consumer receives every fill and narrows it locally.
 *  - de-duplication. Exactly-once used to be emergent — a private monotonic
 *    cursor made a repeat impossible. A shared feed can replay from a cursor
 *    after a reconnect, so the seen-set below is what preserves the property
 *    the filler has always assumed.
 */
export class EventMonitor extends EventEmitter {
	private listening = false
	private configService: FillerConfigService
	private clientManager: ChainClientManager
	private fillerAddress: string
	private logger: Logger
	private orderSource: OrderSource
	private subscriptions: Map<number, Subscription> = new Map()
	private chains: Map<number, ScanTarget> = new Map()
	/** Order ids already delivered, newest last. Bounds the memory a replay can cost. */
	private seen: Set<string> = new Set()
	private seenOrder: string[] = []

	constructor(
		chainConfigs: ChainConfig[],
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		fillerAddress: HexString,
		orderSource?: OrderSource,
	) {
		super()
		this.logger = moduleLogger(configService.loggers, "event-monitor")
		this.configService = configService
		this.clientManager = clientManager
		this.fillerAddress = fillerAddress.toLowerCase()
		this.orderSource = orderSource ?? new SharedOrderSource(configService.loggers)

		chainConfigs.forEach((config) => this.registerChain(config.chainId))
	}

	/** Records the scan target for a chain. Subscribing happens in startListening. */
	private registerChain(chainId: number): void {
		const chain = `EVM-${chainId}`
		this.chains.set(chainId, {
			chain,
			chainId,
			gateway: this.configService.getIntentGatewayAddress(chain),
			rpcUrls: this.configService.getRpcUrls(chain),
		})
	}

	public async startListening(): Promise<void> {
		if (this.listening) return
		this.listening = true

		for (const chainId of this.chains.keys()) {
			try {
				this.subscribe(chainId)
			} catch (error) {
				this.logger.error({ chainId, err: error }, "Failed to subscribe to the order source")
			}
		}
	}

	private subscribe(chainId: number): void {
		const target = this.chains.get(chainId)
		if (!target || this.subscriptions.has(chainId)) return

		this.subscriptions.set(
			chainId,
			this.orderSource.subscribe(target, {
				onOrder: ({ order, transactionHash }) => this.handleOrder(order, transactionHash),
				onFill: ({ commitment, filler, chainId: fillChainId }) => this.handleFill(commitment, filler, fillChainId),
				onError: (error, erroredChain) =>
					this.logger.error({ chainId: erroredChain, err: error }, "Order source reported a scan failure"),
			}),
		)
		this.logger.info({ chainId }, "Subscribed to gateway events")
	}

	private handleOrder(order: Order, transactionHash: string): void {
		const id = order.id
		if (id) {
			// At-least-once delivery: a shared feed that resumes from a cursor can
			// re-deliver, and the fill path has no idempotency of its own.
			if (this.seen.has(id)) {
				this.logger.debug({ orderId: id }, "Duplicate order from the source, ignoring")
				return
			}
			this.seen.add(id)
			this.seenOrder.push(id)
			if (this.seenOrder.length > SEEN_LIMIT) {
				const evicted = this.seenOrder.shift()
				if (evicted) this.seen.delete(evicted)
			}
		}
		this.emit("newOrder", { order, transactionHash })
	}

	private handleFill(commitment: HexString, filler: string, chainId: number): void {
		// Never a topic filter — see the class comment.
		if (filler?.toLowerCase() !== this.fillerAddress) return
		this.logger.info({ chainId, commitment, filler }, "OrderFilled event detected for this filler")
		this.emit("orderFilledOnChain", { commitment, filler, chainId })
	}

	/** Begins watching a chain added after boot. */
	public async addChain(chainId: number): Promise<void> {
		if (this.chains.has(chainId)) {
			throw new Error(`Chain ${chainId} is already monitored`)
		}
		this.registerChain(chainId)
		if (this.listening) this.subscribe(chainId)
	}

	/** Stops watching a chain. The shared loop keeps running for any other filler on it. */
	public async removeChain(chainId: number): Promise<void> {
		this.subscriptions.get(chainId)?.close()
		this.subscriptions.delete(chainId)
		this.chains.delete(chainId)
		this.logger.info({ chainId }, "Stopped monitoring chain")
	}

	/**
	 * Re-subscribes a chain against its current endpoints.
	 *
	 * The endpoints are part of the scan key, so new ones mean a different shared
	 * loop: releasing the old subscription and taking a new one is what moves this
	 * filler across. The old loop keeps running only if another filler still holds it.
	 */
	public async rebuildChain(chainId: number): Promise<void> {
		if (!this.chains.has(chainId)) {
			throw new Error(`Chain ${chainId} is not monitored`)
		}
		this.subscriptions.get(chainId)?.close()
		this.subscriptions.delete(chainId)
		this.registerChain(chainId)
		if (this.listening) this.subscribe(chainId)
		this.logger.info({ chainId }, "Resubscribed chain on new endpoints")
	}

	/** Events dropped because this filler could not keep up with the shared feed. */
	public droppedEvents(): number {
		let total = 0
		for (const subscription of this.subscriptions.values()) total += subscription.dropped
		return total
	}

	public async stopListening(): Promise<void> {
		this.listening = false
		for (const subscription of this.subscriptions.values()) subscription.close()
		this.subscriptions.clear()
		this.chains.clear()
		this.seen.clear()
		this.seenOrder = []
	}
}
