import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { ChainScanner } from "@/scanner/chain-scanner"
import { LoggerContext, type LogSink } from "@/services/Logger"

/**
 * A scan pass holds the mutex, and a tick that finds it held returns without a
 * word. On a 24h mainnet run one chain's passes hung for 38 minutes at a stretch
 * and the log recorded nothing at all — indistinguishable from a chain with no
 * blocks to scan. These pin what the watchdog says, when, and about which phase.
 */
describe("ChainScanner stall watchdog", () => {
	const TARGET = { chain: "EVM-8453", chainId: 8453, gateway: "0xAA" as const, rpcUrls: ["https://a.example"] }
	const TICK_MS = 3_000
	const THRESHOLD_MS = 45_000
	const REPORT_INTERVAL_MS = 60_000

	function capturingContext() {
		const lines: Array<Record<string, unknown>> = []
		const sink: LogSink = {
			write: (line) => {
				try {
					lines.push(JSON.parse(line))
				} catch {
					// pino writes one JSON object per line; nothing else reaches here.
				}
			},
		}
		return { lines, context: new LoggerContext({ sink, level: "debug" }) }
	}

	const stalls = (lines: Array<Record<string, unknown>>) =>
		lines.filter((l) => String(l.msg).startsWith("Block scan pass has outlived"))
	const recoveries = (lines: Array<Record<string, unknown>>) =>
		lines.filter((l) => String(l.msg).startsWith("Block scan pass finished"))

	/** A scanner whose head and log reads resolve only when the test says so. */
	function hangingScanner() {
		const { lines, context } = capturingContext()
		const scanner = new ChainScanner(TARGET, context, TICK_MS)

		let releaseHead: (head: bigint) => void = () => {}
		let releaseLogs: (logs: unknown[]) => void = () => {}
		// biome-ignore lint/suspicious/noExplicitAny: replacing the private client with a stub
		;(scanner as any).quorumClient = {
			getBlockNumber: () => new Promise<bigint>((resolve) => (releaseHead = resolve)),
			getLogs: () => new Promise<unknown[]>((resolve) => (releaseLogs = resolve)),
		}

		return { scanner, lines, head: () => releaseHead, logs: () => releaseLogs }
	}

	beforeEach(() => {
		vi.useFakeTimers()
	})

	afterEach(() => {
		vi.clearAllTimers()
		vi.useRealTimers()
	})

	it("says nothing while a pass is still inside its budget", async () => {
		const { scanner, lines, head } = hangingScanner()
		scanner.start()

		await vi.advanceTimersByTimeAsync(THRESHOLD_MS - TICK_MS)

		expect(stalls(lines)).toHaveLength(0)
		head()(100n)
	})

	it("names the phase a stuck pass is waiting in", async () => {
		const { scanner, lines, head } = hangingScanner()
		scanner.start()

		await vi.advanceTimersByTimeAsync(THRESHOLD_MS + TICK_MS)

		const reported = stalls(lines)
		expect(reported).toHaveLength(1)
		expect(reported[0].phase).toBe("head")
		expect(reported[0].chainId).toBe(8453)
		expect(Number(reported[0].heldMs)).toBeGreaterThanOrEqual(THRESHOLD_MS)
		head()(100n)
	})

	it("reports the log read once the pass gets that far", async () => {
		const { scanner, lines, head, logs } = hangingScanner()
		scanner.start()

		// The first pass only takes the cursor from the head and returns.
		await vi.advanceTimersByTimeAsync(TICK_MS)
		head()(100n)
		await vi.advanceTimersByTimeAsync(0)
		expect(scanner.scannedTo).toBe(99n)

		// The second pass reads the head, then hangs on the range.
		await vi.advanceTimersByTimeAsync(TICK_MS)
		head()(150n)
		await vi.advanceTimersByTimeAsync(THRESHOLD_MS + TICK_MS)

		const reported = stalls(lines)
		expect(reported).toHaveLength(1)
		expect(reported[0].phase).toBe("logs")
		logs()([])
	})

	it("repeats on its own interval rather than on every tick", async () => {
		const { scanner, lines, head } = hangingScanner()
		scanner.start()

		await vi.advanceTimersByTimeAsync(THRESHOLD_MS + TICK_MS)
		expect(stalls(lines)).toHaveLength(1)

		// Many ticks, still inside the first report interval.
		await vi.advanceTimersByTimeAsync(REPORT_INTERVAL_MS - TICK_MS * 2)
		expect(stalls(lines)).toHaveLength(1)

		await vi.advanceTimersByTimeAsync(TICK_MS * 2)
		expect(stalls(lines)).toHaveLength(2)
		head()(100n)
	})

	it("says when a reported pass finally ends", async () => {
		const { scanner, lines, head } = hangingScanner()
		scanner.start()

		await vi.advanceTimersByTimeAsync(THRESHOLD_MS + TICK_MS)
		expect(stalls(lines)).toHaveLength(1)
		expect(recoveries(lines)).toHaveLength(0)

		head()(100n)
		await vi.advanceTimersByTimeAsync(0)

		const done = recoveries(lines)
		expect(done).toHaveLength(1)
		expect(done[0].phase).toBe("head")
		expect(Number(done[0].heldMs)).toBeGreaterThanOrEqual(THRESHOLD_MS)
	})

	it("stays quiet about passes that complete normally", async () => {
		const { lines, context } = capturingContext()
		const scanner = new ChainScanner(TARGET, context, TICK_MS)
		let head = 100n
		// biome-ignore lint/suspicious/noExplicitAny: replacing the private client with a stub
		;(scanner as any).quorumClient = {
			getBlockNumber: async () => head,
			getLogs: async () => [],
		}

		scanner.start()
		await vi.advanceTimersByTimeAsync(TICK_MS)
		head = 120n
		await vi.advanceTimersByTimeAsync(THRESHOLD_MS * 2)

		expect(stalls(lines)).toHaveLength(0)
		expect(recoveries(lines)).toHaveLength(0)
		expect(scanner.scannedTo).toBe(120n)
	})
})
