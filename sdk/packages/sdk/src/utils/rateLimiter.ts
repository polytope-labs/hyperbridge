/**
 * A token bucket, for pacing outbound requests against a server-side rate limit.
 *
 * Rate limits are enforced on the instantaneous rate, not the average, which is the distinction
 * that matters for anything that issues requests in bursts. A block scan that reads two requests
 * per block and then sleeps averages well under one request a second while still firing forty of
 * them inside a few hundred milliseconds — an average a limiter never sees and a burst it rejects.
 *
 * Waiters are served strictly in order. Concurrent callers (a fan-out over every chain's phantom
 * order, say) therefore drain at the configured rate rather than each racing for whichever token
 * happens to be free, and a request queued behind a burst is never starved by one issued later.
 */
export class TokenBucket {
	/** Fractional on purpose: a partial token is real capacity, just not yet a whole request. */
	private tokens: number
	private lastRefill: number
	private readonly waiting: Array<() => void> = []
	private drainTimer: ReturnType<typeof setTimeout> | null = null

	/**
	 * @param ratePerSecond - sustained requests per second.
	 * @param burst - how many requests may go out back-to-back before pacing starts. Defaults to
	 *   one second's worth, which is the shape most limiters police.
	 */
	constructor(
		private readonly ratePerSecond: number,
		private readonly burst: number = ratePerSecond,
	) {
		if (ratePerSecond <= 0) throw new Error(`TokenBucket rate must be positive, got ${ratePerSecond}`)
		this.tokens = burst
		this.lastRefill = Date.now()
	}

	/** Resolves once this caller may send. */
	async acquire(): Promise<void> {
		// The queue check comes first so a caller arriving while others wait cannot jump ahead of
		// them by finding a token the refill just produced.
		if (this.waiting.length === 0 && this.take()) return
		return new Promise<void>((resolve) => {
			this.waiting.push(resolve)
			this.scheduleDrain()
		})
	}

	/** Requests currently waiting on a token. Exposed for tests and diagnostics. */
	get queued(): number {
		return this.waiting.length
	}

	private refill(): void {
		const now = Date.now()
		const elapsed = now - this.lastRefill
		// A clock that went backwards must not mint tokens or freeze the bucket forever.
		if (elapsed <= 0) {
			if (elapsed < 0) this.lastRefill = now
			return
		}
		this.tokens = Math.min(this.burst, this.tokens + (elapsed * this.ratePerSecond) / 1000)
		this.lastRefill = now
	}

	private take(): boolean {
		this.refill()
		if (this.tokens < 1) return false
		this.tokens -= 1
		return true
	}

	private scheduleDrain(): void {
		if (this.drainTimer) return
		this.refill()
		const deficit = 1 - this.tokens
		const waitMs = deficit <= 0 ? 0 : Math.ceil((deficit * 1000) / this.ratePerSecond)
		const timer = setTimeout(() => {
			this.drainTimer = null
			this.drain()
		}, waitMs)
		// A bucket with waiters must not be the reason a process stays alive.
		;(timer as unknown as { unref?: () => void }).unref?.()
		this.drainTimer = timer
	}

	private drain(): void {
		while (this.waiting.length > 0 && this.take()) {
			this.waiting.shift()?.()
		}
		if (this.waiting.length > 0) this.scheduleDrain()
	}
}
