import { EventEmitter } from "events"
import type {
	ChainConfig,
	Order,
	HexString,
} from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { type Logger, moduleLogger } from "@/services/Logger"
import { reconstructOrdersFromLogs, type ReconstructDeps, type ReconstructedOrder } from "@/scanner/reconstruct"
import type { OrderScanner, Subscription } from "@/scanner/types"

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
	private fillerAddress: string
	private logger: Logger
	private orderScanner: OrderScanner
	private subscription?: Subscription
	/** Chain ids this filler is configured for; the scanner may carry more. */
	private chains: Set<number> = new Set()
	/** Order ids already delivered, newest last. Bounds the memory a replay can cost. */
	private seen: Set<string> = new Set()
	private seenOrder: string[] = []

	constructor(
		chainConfigs: ChainConfig[],
		configService: FillerConfigService,
		fillerAddress: HexString,
		orderScanner: OrderScanner,
	) {
		super()
		this.logger = moduleLogger(configService.loggers, "event-monitor")
		this.configService = configService
		this.fillerAddress = fillerAddress.toLowerCase()
		this.orderScanner = orderScanner

		chainConfigs.forEach((config) => this.registerChain(config.chainId))
	}

	/** Marks a chain as one this filler cares about. */
	private registerChain(chainId: number): void {
		this.chains.add(chainId)
	}

	/**
	 * Attaches to the scanner. One subscription covers every chain it carries — a
	 * scanner shared with other fillers may cover more chains than this one is
	 * configured for, so events are matched on `chainId` here.
	 */
	public async startListening(): Promise<void> {
		if (this.listening) return
		this.listening = true

		this.subscription = this.orderScanner.subscribe({
			onOrder: (event) => {
				if (!this.chains.has(event.chainId)) return
				this.handleOrder(event.order, event.transactionHash)
			},
			onFill: (event) => {
				if (!this.chains.has(event.chainId)) return
				this.handleFill(event.commitment, event.filler, event.chainId)
			},
			onError: (error, chainId) =>
				this.logger.error({ chainId, err: error }, "Order scanner reported a scan failure"),
		})
		this.logger.info({ chains: [...this.chains] }, "Subscribed to the order scanner")
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

	/**
	 * Starts matching events for a chain added after boot.
	 *
	 * The scanner must already carry the chain. `Simplex` adds it to a scanner it
	 * owns; when the scanner was supplied by the caller it stays the caller's to
	 * manage, so that another filler reading from it is never surprised by a
	 * chain appearing or vanishing underneath it.
	 */
	public async addChain(chainId: number): Promise<void> {
		if (this.chains.has(chainId)) {
			throw new Error(`Chain ${chainId} is already monitored`)
		}
		if (!this.orderScanner.chains().includes(chainId)) {
			throw new Error(`Chain ${chainId} is not in the order scanner — add it there first`)
		}
		this.chains.add(chainId)
		this.logger.info({ chainId }, "Monitoring chain")
	}

	/** Stops matching events for a chain. The scanner keeps scanning it for others. */
	public async removeChain(chainId: number): Promise<void> {
		this.chains.delete(chainId)
		this.logger.info({ chainId }, "Stopped monitoring chain")
	}

	/**
	 * No-op against a scanner: endpoints belong to the scanner, not to a filler
	 * reading from it. `Simplex.chains.setRpcUrls` swaps the endpoints on a scanner
	 * it owns and rejects the call on one it does not.
	 */
	public async rebuildChain(chainId: number): Promise<void> {
		if (!this.chains.has(chainId)) {
			throw new Error(`Chain ${chainId} is not monitored`)
		}
	}

	/** Events dropped because this filler could not keep up with the scanner. */
	public droppedEvents(): number {
		return this.subscription?.dropped ?? 0
	}

	public async stopListening(): Promise<void> {
		this.listening = false
		this.subscription?.close()
		this.subscription = undefined
		this.chains.clear()
		this.seen.clear()
		this.seenOrder = []
	}
}
