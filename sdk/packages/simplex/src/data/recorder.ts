import { EventEmitter } from "node:events"
import type { EventMonitor } from "@/core/event-monitor"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { ActivityEvent, ActivityInsert, ActivityStore, WalletTx } from "./types"

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

	constructor(
		private store: ActivityStore,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		super()
		this.logger = loggers.get("activity")
	}

	/** Subscribes to the filler's order lifecycle events. Idempotent per monitor. */
	attach(monitor: EventMonitor): void {
		if (this.monitor === monitor) return
		this.detach()
		this.monitor = monitor

		this.subscribe(monitor, "newOrder", ({ order }: { order: { id?: string } }) =>
			this.record({ type: "detected", orderId: order.id ?? null }),
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
			}: {
				orderId?: string
				hash?: string
				volumeUsd?: number
				profitUsd?: number
				chainId?: number
			}) =>
				this.record({
					type: "filled",
					orderId: orderId ?? null,
					txHash: hash ?? null,
					volumeUsd: volumeUsd ?? null,
					profitUsd: profitUsd ?? null,
					chainId: chainId ?? null,
				}),
		)

		this.subscribe(
			monitor,
			"orderExecuted",
			({
				orderId,
				success,
				txHash,
				strategy,
				error,
			}: {
				orderId?: string
				success?: boolean
				txHash?: string
				strategy?: string
				error?: string
			}) =>
				this.record({
					type: "executed",
					orderId: orderId ?? null,
					success: Boolean(success),
					strategy: strategy ?? null,
					txHash: txHash ?? null,
					reason: error ?? null,
				}),
		)

		this.subscribe(monitor, "orderSkipped", ({ orderId, reason }: { orderId?: string; reason?: string }) =>
			this.record({ type: "skipped", orderId: orderId ?? null, reason: reason ?? null }),
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

	/**
	 * Writes an event and emits the stored row. Never rejects — a failed write is
	 * logged and dropped, because the caller is usually a fill in progress.
	 */
	record(event: ActivityInsert): void {
		this.store
			.record(event)
			.then((row: ActivityEvent) => this.emit("event", row))
			.catch((err) => this.logger.warn({ err, type: event.type }, "Failed to record activity event"))
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

	recordWalletTx(tx: Omit<WalletTx, "id" | "ts">): Promise<void> {
		return this.store.recordWalletTx(tx)
	}

	walletTxs(limit?: number): Promise<WalletTx[]> {
		return this.store.walletTxs(limit)
	}
}
