import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { TokenBucket } from "@/utils/rateLimiter"

describe("TokenBucket", () => {
	beforeEach(() => vi.useFakeTimers())
	afterEach(() => vi.useRealTimers())

	/** Acquires `count` permits concurrently, resolving each into an array as it is granted. */
	function acquireAll(bucket: TokenBucket, count: number): number[] {
		const granted: number[] = []
		for (let i = 0; i < count; i++) {
			void bucket.acquire().then(() => granted.push(i))
		}
		return granted
	}

	it("lets a burst through immediately, up to the bucket's capacity", async () => {
		const bucket = new TokenBucket(10)
		const granted = acquireAll(bucket, 15)

		await vi.advanceTimersByTimeAsync(0)

		expect(granted).toHaveLength(10)
	})

	it("paces the rest at the configured rate", async () => {
		const bucket = new TokenBucket(10)
		const granted = acquireAll(bucket, 20)

		await vi.advanceTimersByTimeAsync(0)
		expect(granted).toHaveLength(10)

		// A tenth of a second is one token at 10/s.
		await vi.advanceTimersByTimeAsync(100)
		expect(granted).toHaveLength(11)

		await vi.advanceTimersByTimeAsync(500)
		expect(granted).toHaveLength(16)

		await vi.advanceTimersByTimeAsync(400)
		expect(granted).toHaveLength(20)
		expect(bucket.queued).toBe(0)
	})

	// The fan-out this exists for is concurrent — one offchain read per configured chain, issued at
	// once — so a caller that arrives late must not be served before one already waiting.
	it("grants waiters in the order they arrived", async () => {
		const bucket = new TokenBucket(4)
		const granted = acquireAll(bucket, 12)

		await vi.advanceTimersByTimeAsync(3000)

		expect(granted).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])
	})

	// Without this a caller arriving on an idle bucket takes the token a queued one was waiting for,
	// and a steady stream of new callers starves the queue indefinitely.
	it("does not let a fresh caller overtake a queue", async () => {
		const bucket = new TokenBucket(2)
		const early = acquireAll(bucket, 4)
		await vi.advanceTimersByTimeAsync(0)
		expect(early).toHaveLength(2)

		let lateGranted = false
		void bucket.acquire().then(() => {
			lateGranted = true
		})

		await vi.advanceTimersByTimeAsync(500)
		expect(early).toHaveLength(3)
		expect(lateGranted).toBe(false)

		await vi.advanceTimersByTimeAsync(1000)
		expect(early).toHaveLength(4)
		expect(lateGranted).toBe(true)
	})

	it("refills no further than the burst while idle", async () => {
		const bucket = new TokenBucket(5)

		// Ten seconds of idling is fifty tokens' worth of elapsed time; the bucket holds five.
		await vi.advanceTimersByTimeAsync(10_000)

		const granted = acquireAll(bucket, 10)
		await vi.advanceTimersByTimeAsync(0)

		expect(granted).toHaveLength(5)
	})

	it("takes a separate burst allowance when one is given", async () => {
		const bucket = new TokenBucket(10, 2)
		const granted = acquireAll(bucket, 5)

		await vi.advanceTimersByTimeAsync(0)
		expect(granted).toHaveLength(2)

		await vi.advanceTimersByTimeAsync(300)
		expect(granted).toHaveLength(5)
	})

	it("rejects a rate that would never grant anything", () => {
		expect(() => new TokenBucket(0)).toThrow("must be positive")
	})
})
