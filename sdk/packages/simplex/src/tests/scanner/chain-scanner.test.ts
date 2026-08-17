import { describe, expect, it } from "vitest"
import { ChainScanner } from "@/scanner/chain-scanner"

/**
 * The publish loops yield to consumers every 250 events, and yields are awaits —
 * the exact points where stop() interleaves. These pin the two halves of stop()'s
 * contract across a synchronous burst larger than one chunk: nothing is delivered
 * after stop() resolves, and the cursor never advances past a range whose
 * delivery was cut short (so a rescan sees it, preserving at-least-once).
 */
describe("ChainScanner stop() during a publish burst", () => {
	const TARGET = { chain: "EVM-1", chainId: 1, gateway: "0xAA" as const, rpcUrls: ["https://x.example"] }

	function fillLogs(count: number) {
		return Array.from({ length: count }, (_, i) => ({
			eventName: "OrderFilled",
			args: { commitment: `0x${i.toString(16).padStart(64, "0")}`, filler: "0xF" },
			blockNumber: 100n,
			blockHash: "0xblock",
			logIndex: i,
		}))
	}

	async function burstingScanner(count: number) {
		const scanner = new ChainScanner(TARGET)
		let head = 99n
		// biome-ignore lint/suspicious/noExplicitAny: replacing the private client with a stub
		;(scanner as any).quorumClient = {
			getBlockNumber: async () => head,
			getLogs: async () => fillLogs(count),
		}
		const seen: string[] = []
		scanner.onFill((event) => seen.push(event.commitment))

		// First scan only initialises the cursor from the head.
		// biome-ignore lint/suspicious/noExplicitAny: driving the private loop directly
		await (scanner as any).scan()
		expect(scanner.scannedTo).toBe(98n)

		head = 150n
		// biome-ignore lint/suspicious/noExplicitAny: driving the private loop directly
		const scanDone = (scanner as any).scan() as Promise<void>
		return { scanner, seen, scanDone }
	}

	it("delivers nothing after stop() resolves, and leaves the cursor for a rescan", async () => {
		const { scanner, seen, scanDone } = await burstingScanner(600)

		// Let the first chunk publish, then stop mid-burst.
		await new Promise((resolve) => setImmediate(resolve))
		await Promise.all([scanner.stop(), scanDone])

		const deliveredAtStop = seen.length
		expect(deliveredAtStop).toBeGreaterThan(0) // the burst had started
		expect(deliveredAtStop).toBeLessThan(600) // and was cut short at a yield

		// Anything still queued anywhere would land within a few macrotasks.
		await new Promise((resolve) => setImmediate(resolve))
		await new Promise((resolve) => setImmediate(resolve))
		expect(seen.length).toBe(deliveredAtStop)

		// The range was not fully delivered, so it must not be marked scanned.
		expect(scanner.scannedTo).toBe(98n)
	})

	it("advances the cursor when the burst completes undisturbed", async () => {
		const { scanner, seen, scanDone } = await burstingScanner(600)
		await scanDone
		expect(seen.length).toBe(600)
		expect(scanner.scannedTo).toBe(150n)
		await scanner.stop()
	})
})
