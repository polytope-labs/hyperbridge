import { EventEmitter } from "node:events"
import type { HexString, Order } from "@hyperbridge/sdk"
import type { EventMonitor } from "@/core/event-monitor"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type {
	ActivityEvent,
	ActivityInsert,
	ActivityStore,
	OrderHistoryPage,
	OrderLeg,
	OrderSummary,
	WalletTx,
} from "./types"

/** Symbol and decimals for a token on a chain, as far as the filler knows them. */
export interface TokenDescription {
	symbol: string | null
	decimals: number | null
}

/** Resolves display metadata for `token` (20-byte address) on `chain` (state machine id). */
export type TokenDescriber = (chain: string, token: string) => Promise<TokenDescription>

export interface ActivityRecorderOptions {
	/**
	 * Enriches order legs with symbol/decimals so the feed can show "19,585 USDC"
	 * rather than a raw integer. Omit to record addresses and raw amounts only.
	 */
	describeToken?: TokenDescriber
}

/** How many order summaries to keep for attaching to later events of the same order. */
const SUMMARY_CACHE = 2_000

const ZERO_BYTES32 = `0x${"0".repeat(64)}`

/** Last 20 bytes of a bytes32 (or an address as-is), lowercased. */
export function toBytes20(value: string): string {
	const hex = value.toLowerCase()
	return hex.length > 42 ? `0x${hex.slice(-40)}` : hex
}

/**
 * The referrer recorded for an order: the whole 32-byte graffiti tag (it may
 * encode a name such as "HyperFX" as padded ASCII, or an address in its low 20
 * bytes), or null when it is empty or names the placer (the indexer's
 * self-referral rule).
 */
export function referrerFrom(graffiti: string | undefined, user: string): string | null {
	if (!graffiti) return null
	const tag = graffiti.toLowerCase()
	if (tag === ZERO_BYTES32) return null
	return toBytes20(tag) === user ? null : tag
}

/**
 * Bridges the filler's order lifecycle onto an {@link ActivityStore}.
 *
 * The store used to subscribe to the `EventMonitor` itself, which meant a
 * consumer implementing the data interface also had to know about the filler's
 * internal event names. Ownership is inverted here: the recorder listens, the
 * store just receives rows.
 *
 * Writes are fire-and-forget — the activity feed is observability, and a failed
 * insert must never propagate back into the fill path that emitted it. Each
 * stored row is re-emitted as `event` for live consumers (the UI's SSE stream,
 * and `Simplex`'s `activity` event).
 */
export class ActivityRecorder extends EventEmitter {
	private logger: Logger
	private monitor?: EventMonitor
	// biome-ignore lint/suspicious/noExplicitAny: EventEmitter listeners are untyped by construction
	private handlers: Array<[string, (payload: any) => void]> = []
	private describeToken?: TokenDescriber
	/** Order summaries by order id, so filled/skipped rows carry the order too. Insertion-ordered for eviction. */
	private summaries = new Map<string, OrderSummary>()

	constructor(
		private store: ActivityStore,
		loggers: LoggerContext = defaultLoggerContext(),
		options: ActivityRecorderOptions = {},
	) {
		super()
		this.logger = loggers.get("activity")
		this.describeToken = options.describeToken
	}

	/** Subscribes to the filler's order lifecycle events. Idempotent per monitor. */
	attach(monitor: EventMonitor): void {
		if (this.monitor === monitor) return
		this.detach()
		this.monitor = monitor

		this.subscribe(
			monitor,
			"newOrder",
			({ order, transactionHash, graffiti }: { order: Order; transactionHash?: string; graffiti?: HexString }) =>
				void this.summarize(order, transactionHash, graffiti).then((summary) => {
					if (order.id) this.remember(order.id, summary)
					this.record({ type: "detected", orderId: order.id ?? null, order: summary })
				}),
		)

		this.subscribe(
			monitor,
			"orderFilled",
			({
				orderId,
				hash,
				volumeUsd,
				profitUsd,
				chainId,
				commitment,
			}: {
				orderId?: string
				hash?: string
				volumeUsd?: number
				profitUsd?: number
				chainId?: number
				commitment?: string
			}) => {
				// A bid is not a fill: the order settles when its OrderFilled log is
				// observed (below), so a bid's "filled" is recorded from `orderExecuted`
				// as a `bid` row instead.
				if (commitment) return
				this.record({
					type: "filled",
					orderId: orderId ?? null,
					txHash: hash ?? null,
					volumeUsd: volumeUsd ?? null,
					profitUsd: profitUsd ?? null,
					chainId: chainId ?? null,
					order: this.summaryFor(orderId),
				})
			},
		)

		this.subscribe(
			monitor,
			"orderFillObserved",
			({
				commitment,
				filler,
				chainId,
				txHash,
				ours,
			}: {
				commitment: string
				filler: string
				chainId: number
				txHash?: string
				ours: boolean
			}) =>
				void this.settle(commitment, filler, chainId, txHash, ours).catch((err) =>
					this.logger.warn({ err, commitment }, "Failed to record an observed fill"),
				),
		)

		this.subscribe(
			monitor,
			"orderExecuted",
			({
				orderId,
				success,
				txHash,
				strategy,
				commitment,
				error,
			}: {
				orderId?: string
				success?: boolean
				txHash?: string
				strategy?: string
				commitment?: string
				error?: string
			}) =>
				this.record({
					// An accepted bid: the order is now Hyperbridge's to award.
					type: commitment && success ? "bid" : "executed",
					orderId: orderId ?? null,
					success: Boolean(success),
					strategy: strategy ?? null,
					txHash: txHash ?? null,
					reason: error ?? null,
					order: this.summaryFor(orderId),
				}),
		)

		this.subscribe(monitor, "orderSkipped", ({ orderId, reason }: { orderId?: string; reason?: string }) =>
			this.record({
				type: "skipped",
				orderId: orderId ?? null,
				reason: reason ?? null,
				order: this.summaryFor(orderId),
			}),
		)

		this.subscribe(
			monitor,
			"rebalanceExecuted",
			({
				transferCount,
				executedCount,
				success,
				error,
			}: {
				transferCount?: number
				executedCount?: number
				success?: boolean
				error?: string
			}) =>
				this.record({
					type: "rebalance",
					success: Boolean(success),
					reason:
						error ?? (transferCount !== undefined ? `${executedCount}/${transferCount} transfers executed` : null),
				}),
		)
	}

	private subscribe<T>(monitor: EventMonitor, event: string, handler: (payload: T) => void): void {
		monitor.on(event, handler)
		this.handlers.push([event, handler])
	}

	/** Unsubscribes from the attached monitor. Safe to call when never attached. */
	detach(): void {
		if (!this.monitor) return
		for (const [event, handler] of this.handlers) {
			this.monitor.off(event, handler)
		}
		this.handlers = []
		this.monitor = undefined
	}

	private remember(orderId: string, summary: OrderSummary): void {
		this.summaries.set(orderId, summary)
		if (this.summaries.size > SUMMARY_CACHE) {
			const oldest = this.summaries.keys().next().value
			if (oldest !== undefined) this.summaries.delete(oldest)
		}
	}

	private summaryFor(orderId: string | undefined): OrderSummary | null {
		return orderId ? (this.summaries.get(orderId) ?? null) : null
	}

	/**
	 * Settles an order from its on-chain fill: `filled` when this filler won it,
	 * `lost` (reason = the winner) when a rival did. A rival's fill is recorded
	 * only for orders this filler has rows for — every fill on the chain passes
	 * through here, and the feed is about this filler's orders.
	 */
	private async settle(
		commitment: string,
		filler: string,
		chainId: number,
		txHash: string | undefined,
		ours: boolean,
	): Promise<void> {
		if (!ours && !this.summaries.has(commitment) && !(await this.store.knowsOrder(commitment))) return
		this.record({
			type: ours ? "filled" : "lost",
			orderId: commitment,
			chainId,
			txHash: txHash ?? null,
			reason: ours ? null : filler,
			order: this.summaryFor(commitment),
		})
	}

	/**
	 * Captures the order for the feed. Token metadata lookups may hit the chain;
	 * a failure degrades that leg to address + raw amount rather than dropping the row.
	 */
	private async summarize(order: Order, transactionHash?: string, graffiti?: HexString): Promise<OrderSummary> {
		const legs = async (chain: string, assets: Array<{ token: string; amount: bigint }>): Promise<OrderLeg[]> =>
			Promise.all(
				assets.map(async ({ token, amount }) => {
					const address = toBytes20(token)
					let described: TokenDescription = { symbol: null, decimals: null }
					if (this.describeToken) {
						try {
							described = await this.describeToken(chain, address)
						} catch (err) {
							this.logger.debug({ err, chain, token: address }, "Token metadata lookup failed")
						}
					}
					return { token: address, amount: amount.toString(), ...described }
				}),
			)
		const user = toBytes20(order.user)
		const [inputs, outputs] = await Promise.all([
			legs(order.source, order.inputs),
			legs(order.destination, order.output.assets),
		])
		return {
			user,
			source: order.source,
			destination: order.destination,
			placedTxHash: transactionHash ?? null,
			referrer: referrerFrom(graffiti, user),
			inputs,
			outputs,
			deadline: order.deadline.toString(),
		}
	}
	/**
	 * Writes an event and emits the stored row. Never rejects — a failed write is
	 * logged and dropped, because the caller is usually a fill in progress.
	 */
	record(event: ActivityInsert): void {
		this.store.record(event).then(
			(row: ActivityEvent) => {
				// Separated from the write's catch: a throwing listener is a consumer
				// bug and must not be logged as a failed store write.
				try {
					this.emit("event", row)
				} catch (err) {
					this.logger.error({ err, type: event.type }, "Activity listener threw")
				}
			},
			(err) => this.logger.warn({ err, type: event.type }, "Failed to record activity event"),
		)
	}

	// ─── Read-through to the store ──────────────────────────────────────────
	// So callers that need both the live stream and history (the operator UI,
	// `Simplex.activity`) hold one object instead of a recorder plus a store.

	recent(limit?: number, beforeId?: number): Promise<ActivityEvent[]> {
		return this.store.recent(limit, beforeId)
	}

	fills(limit?: number): Promise<ActivityEvent[]> {
		return this.store.fills(limit)
	}

	orderHistory(page: number, pageSize: number): Promise<OrderHistoryPage> {
		return this.store.orderHistory(page, pageSize)
	}

	recordWalletTx(tx: Omit<WalletTx, "id" | "ts">): Promise<void> {
		return this.store.recordWalletTx(tx)
	}

	walletTxs(limit?: number): Promise<WalletTx[]> {
		return this.store.walletTxs(limit)
	}
}
