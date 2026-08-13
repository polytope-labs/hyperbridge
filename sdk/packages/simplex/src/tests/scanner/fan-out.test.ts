import { describe, expect, it, vi } from "vitest"
import { FanOut, RefCounted } from "@/scanner/fan-out"

/** Lets the microtask-based drain run to completion. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))

describe("FanOut", () => {
	it("delivers every event to every consumer", async () => {
		const fan = new FanOut<number>("test")
		const a: number[] = []
		const b: number[] = []
		fan.add((n) => a.push(n))
		fan.add((n) => b.push(n))

		fan.publish(1)
		fan.publish(2)
		await settle()

		expect(a).toEqual([1, 2])
		expect(b).toEqual([1, 2])
	})

	it("stops delivering to a closed consumer without affecting the rest", async () => {
		const fan = new FanOut<number>("test")
		const a: number[] = []
		const b: number[] = []
		const first = fan.add((n) => a.push(n))
		fan.add((n) => b.push(n))

		fan.publish(1)
		await settle()
		first.close()
		fan.publish(2)
		await settle()

		expect(a).toEqual([1])
		expect(b).toEqual([1, 2])
		expect(fan.size).toBe(1)
	})

	it("drops the oldest events for a consumer that falls behind, and counts them", async () => {
		// Capacity 2 with a consumer that never drains: the whole point is that a slow
		// filler loses events rather than applying backpressure to the shared scanner.
		const fan = new FanOut<number>("test", undefined, 2)
		const seen: number[] = []
		const consumer = fan.add((n) => seen.push(n))

		for (let i = 1; i <= 5; i++) fan.publish(i)
		await settle()

		expect(consumer.dropped).toBe(3)
		// The newest survive — an order from a thousand events ago is not fillable.
		expect(seen.at(-1)).toBe(5)
	})

	it("does not let one consumer's throw reach the producer or the other consumers", async () => {
		const fan = new FanOut<number>("test")
		const good: number[] = []
		fan.add(() => {
			throw new Error("consumer exploded")
		})
		fan.add((n) => good.push(n))

		expect(() => fan.publish(1)).not.toThrow()
		await settle()
		expect(good).toEqual([1])
	})

	it("publishes synchronously without waiting on consumers", () => {
		const fan = new FanOut<number>("test")
		let delivered = 0
		fan.add(() => delivered++)

		fan.publish(1)
		// Delivery is deferred to a microtask, so the scan loop is never blocked by it.
		expect(delivered).toBe(0)
	})
})

describe("RefCounted", () => {
	it("creates once and stops only when the last holder releases", async () => {
		const refs = new RefCounted<{ stop(): void }>("test")
		const stop = vi.fn()
		const create = vi.fn(() => ({ stop }))

		const first = refs.acquire("k", create)
		const second = refs.acquire("k", create)
		expect(create).toHaveBeenCalledTimes(1)
		expect(first.value).toBe(second.value)

		first.release()
		expect(stop).not.toHaveBeenCalled()

		second.release()
		expect(stop).toHaveBeenCalledTimes(1)
		expect(refs.keys()).toEqual([])
	})

	it("keeps different keys independent", () => {
		const refs = new RefCounted<{ stop(): void }>("test")
		const a = refs.acquire("a", () => ({ stop: vi.fn() }))
		const b = refs.acquire("b", () => ({ stop: vi.fn() }))
		expect(a.value).not.toBe(b.value)
		expect(refs.keys().sort()).toEqual(["a", "b"])
	})

	it("ignores a double release rather than dropping someone else's refcount", () => {
		const refs = new RefCounted<{ stop(): void }>("test")
		const stop = vi.fn()
		const first = refs.acquire("k", () => ({ stop }))
		const second = refs.acquire("k", () => ({ stop }))

		first.release()
		first.release()

		expect(stop).not.toHaveBeenCalled()
		second.release()
		expect(stop).toHaveBeenCalledTimes(1)
	})

	it("recreates after the last release", () => {
		const refs = new RefCounted<{ stop(): void }>("test")
		const create = vi.fn(() => ({ stop: vi.fn() }))
		refs.acquire("k", create).release()
		refs.acquire("k", create)
		expect(create).toHaveBeenCalledTimes(2)
	})
})
