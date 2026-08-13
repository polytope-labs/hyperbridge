import { getLogger, type Logger, type LoggerContext, defaultLoggerContext } from "@/services/Logger"

/**
 * Delivers each event to every consumer without letting any of them slow the
 * producer down.
 *
 * One shared scan loop feeding many fillers has a failure mode a private loop
 * never had: if delivery were synchronous and a consumer blocked — a slow
 * strategy, a paused filler, a host callback doing I/O — every other filler
 * would stall behind it. So each consumer gets a bounded queue drained on its
 * own microtask, and a consumer that falls behind loses its oldest events
 * instead of applying backpressure upstream.
 *
 * Dropping is deliberate and matches what the filler already does: orders that
 * arrive while it is paused are dropped, not queued, and resuming does not
 * replay them. `dropped` is exposed so the loss is visible rather than silent.
 */
export class FanOut<T> {
	private readonly consumers = new Set<Consumer<T>>()
	private readonly logger: Logger

	constructor(
		private readonly label: string,
		loggers: LoggerContext = defaultLoggerContext(),
		private readonly capacity = 1_000,
	) {
		this.logger = loggers.get("scanner")
	}

	get size(): number {
		return this.consumers.size
	}

	/** Registers a consumer. The returned handle stops delivery to it. */
	add(deliver: (event: T) => void): Consumer<T> {
		const consumer = new Consumer<T>(deliver, this.capacity, this.label, this.logger, () => {
			this.consumers.delete(consumer)
		})
		this.consumers.add(consumer)
		return consumer
	}

	/** Queues an event for every consumer. Never throws, never blocks. */
	publish(event: T): void {
		for (const consumer of this.consumers) consumer.push(event)
	}
}

export class Consumer<T> {
	private queue: T[] = []
	private draining = false
	private closed = false
	private droppedCount = 0

	constructor(
		private readonly deliver: (event: T) => void,
		private readonly capacity: number,
		private readonly label: string,
		private readonly logger: Logger,
		private readonly onClose: () => void,
	) {}

	get dropped(): number {
		return this.droppedCount
	}

	push(event: T): void {
		if (this.closed) return
		if (this.queue.length >= this.capacity) {
			// Drop the oldest: a filler that has fallen a thousand events behind is
			// better served by the newest orders, which are the only ones still
			// fillable, than by working through a backlog of expired ones.
			this.queue.shift()
			this.droppedCount++
			if (this.droppedCount === 1 || this.droppedCount % 100 === 0) {
				this.logger.warn(
					{ source: this.label, dropped: this.droppedCount, capacity: this.capacity },
					"Consumer is behind the shared scanner; dropping oldest events",
				)
			}
		}
		this.queue.push(event)
		// queueMicrotask, not a direct call: an async function still runs synchronously
		// up to its first await, so calling drain() here would deliver the first event
		// inside publish() — putting consumer code back on the shared scan loop, which
		// is exactly what this class exists to prevent.
		if (!this.draining) {
			this.draining = true
			queueMicrotask(() => void this.drain())
		}
	}

	/**
	 * Drains on a microtask so `publish` returns immediately, and so a consumer
	 * that throws cannot unwind into the scan loop that produced the event.
	 */
	private async drain(): Promise<void> {
		try {
			while (this.queue.length > 0 && !this.closed) {
				const event = this.queue.shift() as T
				try {
					this.deliver(event)
				} catch (err) {
					this.logger.error({ source: this.label, err }, "Consumer threw while handling a scanner event")
				}
				// Yield so one consumer's backlog cannot monopolise the loop.
				await Promise.resolve()
			}
		} finally {
			this.draining = false
		}
	}

	close(): void {
		if (this.closed) return
		this.closed = true
		this.queue = []
		this.onClose()
	}
}

/**
 * Keeps one shared resource alive per key for as long as anyone holds it.
 *
 * Both scanners are per-key singletons: a scan loop per (chain, gateway,
 * endpoints), and a poller per Hyperbridge endpoint. Fillers come and go —
 * `chains.add`/`chains.remove` at runtime, whole instances starting and
 * stopping — so lifetime is refcounted rather than owned by whoever happened to
 * ask first.
 */
export class RefCounted<T extends { stop(): void | Promise<void> }> {
	private readonly entries = new Map<string, { value: T; refs: number }>()
	private readonly logger = getLogger("scanner")

	constructor(private readonly kind: string) {}

	acquire(key: string, create: () => T): { value: T; release: () => void } {
		let entry = this.entries.get(key)
		if (!entry) {
			entry = { value: create(), refs: 0 }
			this.entries.set(key, entry)
			this.logger.info({ kind: this.kind, key }, "Started shared scanner")
		}
		entry.refs++

		let released = false
		return {
			value: entry.value,
			release: () => {
				if (released) return
				released = true
				const current = this.entries.get(key)
				if (!current) return
				current.refs--
				if (current.refs > 0) return
				this.entries.delete(key)
				this.logger.info({ kind: this.kind, key }, "Stopped shared scanner (last consumer released)")
				void Promise.resolve(current.value.stop()).catch((err) =>
					this.logger.warn({ kind: this.kind, key, err }, "Shared scanner failed to stop cleanly"),
				)
			},
		}
	}

	/** Keys currently held. */
	keys(): string[] {
		return [...this.entries.keys()]
	}

	/** Live handle for a key, if any consumer holds it. */
	peek(key: string): T | undefined {
		return this.entries.get(key)?.value
	}
}
