import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { scanKey, type ScanTarget } from "@/scanner/types"
import { SharedOrderSource, sharedScanners } from "@/scanner/registry"

/**
 * The point of the shared scanner: N fillers watching a chain cost one chain's
 * worth of RPC, not N. These tests pin the sharing key and the refcounted
 * lifetime; the scan loop itself never runs here because no endpoint answers,
 * which is fine — what matters is how many loops exist.
 */

const GATEWAY = "0xAbCd000000000000000000000000000000000001" as HexString

function target(overrides: Partial<ScanTarget> = {}): ScanTarget {
	return {
		chain: "EVM-8453",
		chainId: 8453,
		gateway: GATEWAY,
		rpcUrls: ["https://a.example", "https://b.example"],
		...overrides,
	}
}

const noop = { onOrder: vi.fn(), onFill: vi.fn() }

describe("scanKey", () => {
	it("is insensitive to endpoint order and gateway casing", () => {
		expect(scanKey(target({ rpcUrls: ["https://b.example", "https://a.example"] }))).toBe(scanKey(target()))
		expect(scanKey(target({ gateway: GATEWAY.toLowerCase() as HexString }))).toBe(scanKey(target()))
	})

	it("separates different endpoint sets on the same chain", () => {
		// A quorum's claim is that *these* endpoints agree. Folding two consumers with
		// different endpoint sets onto one loop would hand one of them a consensus it
		// never asked for.
		expect(scanKey(target({ rpcUrls: ["https://a.example"] }))).not.toBe(scanKey(target()))
	})

	it("separates a redeployed gateway on the same chain", () => {
		const other = "0xAbCd000000000000000000000000000000000002" as HexString
		expect(scanKey(target({ gateway: other }))).not.toBe(scanKey(target()))
	})
})

describe("SharedOrderSource", () => {
	it("runs one scan loop for many consumers on the same target", () => {
		const source = new SharedOrderSource()
		const before = sharedScanners.chains().length

		const a = source.subscribe(target(), noop)
		const b = source.subscribe(target(), noop)
		const c = source.subscribe(target({ rpcUrls: ["https://b.example", "https://a.example"] }), noop)

		// Three fillers, one loop — including the one that listed its endpoints differently.
		expect(sharedScanners.chains().length).toBe(before + 1)

		a.close()
		b.close()
		c.close()
	})

	it("keeps the loop alive until the last consumer releases", () => {
		const source = new SharedOrderSource()
		const key = scanKey(target())

		const a = source.subscribe(target(), noop)
		const b = source.subscribe(target(), noop)
		expect(sharedScanners.chains()).toContain(key)

		a.close()
		expect(sharedScanners.chains()).toContain(key)

		b.close()
		expect(sharedScanners.chains()).not.toContain(key)
	})

	it("runs separate loops for different endpoint sets", () => {
		const source = new SharedOrderSource()
		const own = source.subscribe(target(), noop)
		const other = source.subscribe(target({ rpcUrls: ["https://c.example"] }), noop)

		expect(sharedScanners.chains().filter((k) => k.startsWith("8453:")).length).toBe(2)

		own.close()
		other.close()
	})

	it("ignores a double close rather than releasing someone else's hold", () => {
		const source = new SharedOrderSource()
		const key = scanKey(target())
		const a = source.subscribe(target(), noop)
		const b = source.subscribe(target(), noop)

		a.close()
		a.close()
		expect(sharedScanners.chains()).toContain(key)

		b.close()
		expect(sharedScanners.chains()).not.toContain(key)
	})

	it("fans one scanned order out to every subscriber", async () => {
		const source = new SharedOrderSource()
		const seenA: string[] = []
		const seenB: string[] = []
		const a = source.subscribe(target(), { onOrder: (e) => seenA.push(e.order.id!), onFill: vi.fn() })
		const b = source.subscribe(target(), { onOrder: (e) => seenB.push(e.order.id!), onFill: vi.fn() })

		// Publish through the live scanner both subscriptions resolved to.
		const scanner = sharedScanners.chainScanner(target())
		expect(scanner).toBeDefined()
		// biome-ignore lint/suspicious/noExplicitAny: reaching into the shared loop to inject one event
		;(scanner as any).orders.publish({
			order: { id: "0xorder" },
			transactionHash: "0xtx",
			blockNumber: 1n,
			blockHash: "0xblock",
			logIndex: 0,
			chain: "EVM-8453",
			chainId: 8453,
		})
		await new Promise((resolve) => setTimeout(resolve, 0))

		expect(seenA).toEqual(["0xorder"])
		expect(seenB).toEqual(["0xorder"])

		a.close()
		b.close()
	})
})
