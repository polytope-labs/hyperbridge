import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { LimitOrderService } from "./limit-orders"
import { FALLBACK_HEARTBEAT_INTERVAL_MS } from "./types"

/** How often to sweep orders that have outlived their TTL off the book. */
const EXPIRY_SWEEP_MS = 30_000

export interface LifecycleOptions {
	/** How often to check the orderbook's copy of the operator's orders. */
	reconcileIntervalSecs: number
}

/**
 * Keeps the operator's postings alive while the filler runs.
 *
 * Three jobs on three clocks, all of them work {@link LimitOrderService} already
 * knows how to do. The heartbeat stops the orderbook suspending the solver, the
 * expiry sweep takes orders that have outlived their TTL off the book, and
 * reconciliation repairs what a crash or an unanswered request left behind.
 *
 * Nothing renews. An order's TTL is its whole life: when it runs out the posting
 * lapses and the order is done, and the operator posts a fresh one if they still
 * want the depth.
 *
 * A pass that throws is logged and the clock carries on: every one of them is
 * safe to run again, and an orderbook that is briefly unreachable must not take
 * the filler down with it.
 */
export class LimitOrderLifecycle {
	private logger: Logger
	private timers: NodeJS.Timeout[] = []

	constructor(
		private readonly service: LimitOrderService,
		private readonly options: LifecycleOptions,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("limit-orders")
	}

	/**
	 * Starts the clocks and kicks off the first reconciliation.
	 *
	 * That first pass is not awaited. It is a round trip to the orderbook for
	 * every page of the solver's orders, and boot has no reason to wait on it.
	 */
	async start(): Promise<void> {
		if (this.timers.length > 0) return

		// An orderbook that is down at boot must not stop the filler starting: it
		// prices from the local limit orders either way, and the clocks below are
		// what bring the postings back once it answers again.
		const heartbeatMs = await this.service.heartbeatIntervalMs().catch((err) => {
			this.logger.warn({ err }, "Could not read the orderbook's heartbeat interval; using the fallback")
			return FALLBACK_HEARTBEAT_INTERVAL_MS
		})
		const reconcileMs = this.options.reconcileIntervalSecs * 1000

		this.every(heartbeatMs, "heartbeat", () => this.service.heartbeat())
		this.every(EXPIRY_SWEEP_MS, "expiry", () => this.expire())
		this.every(reconcileMs, "reconciliation", () => this.reconcile())

		this.logger.info({ heartbeatMs, expiryMs: EXPIRY_SWEEP_MS, reconcileMs }, "Orderbook lifecycle started")
		void this.run("reconciliation", () => this.reconcile())
	}

	stop(): void {
		this.timers.forEach((timer) => clearInterval(timer))
		this.timers = []
	}

	/** Takes orders that have outlived their TTL off the book. */
	private async expire(): Promise<void> {
		const expired = await this.service.expireStale()
		if (expired > 0) this.logger.info({ expired }, "Withdrew limit orders that had outlived their expiry")
	}

	private async reconcile(): Promise<void> {
		const report = await this.service.reconcile()
		if (report.cancelled || report.reposted || report.underFunded) {
			this.logger.info(report, "Reconciled the operator's limit orders against the orderbook")
		}
	}

	/**
	 * Runs `pass` on an interval, skipping a tick while the previous one is still
	 * going. An orderbook slower than the interval would otherwise have passes
	 * overlapping, and two reconciliations at once would each repair the other's
	 * work in progress.
	 */
	private every(intervalMs: number, what: string, pass: () => Promise<unknown>): void {
		let running = false
		const timer = setInterval(() => {
			if (running) return
			running = true
			void this.run(what, pass).finally(() => {
				running = false
			})
		}, intervalMs)
		this.timers.push(timer)
	}

	/** One pass, whose failure is the clock's business and nobody else's. */
	private run(what: string, pass: () => Promise<unknown>): Promise<void> {
		return pass().then(
			() => {},
			(err) => this.logger.warn({ err }, `Orderbook ${what} failed; trying again next tick`),
		)
	}
}
