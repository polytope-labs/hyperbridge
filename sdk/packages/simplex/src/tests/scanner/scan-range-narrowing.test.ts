import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { ChainScanner } from "@/scanner/chain-scanner"
import { LoggerContext } from "@/services/Logger"

/**
 * Free endpoints cap `eth_getLogs` ranges. On a mainnet Arbitrum run, three of
 * seven refused 100 blocks while the first pass after boot was already 117, and
 * the cursor never moves past a range it failed to read — so at a fixed span
 * the chain stayed stuck, the range growing to the full span and never shrinking
 * back under the caps. These pin the way out: narrow while reads fail, widen
 * once they succeed, and never skip a block doing either.
 */
describe("ChainScanner range narrowing", () => {
	const TARGET = { chain: "EVM-42161", chainId: 42161, gateway: "0xAA" as const, rpcUrls: ["https://x.example"] }
	const START = 1_000_000n

	/** A scanner whose quorum refuses any range wider than `cap`, as `toBlock - fromBlock`. */
	function cappedScanner(cap: { value: bigint }) {
		const scanner = new ChainScanner(TARGET, new LoggerContext({ sink: { write() {} } }))
		let head = START
		const reads: Array<{ from: bigint; to: bigint; ok: boolean }> = []
		// biome-ignore lint/suspicious/noExplicitAny: replacing the private client with a stub
		;(scanner as any).quorumClient = {
			getBlockNumber: async () => head,
			getLogs: async ({ fromBlock, toBlock }: { fromBlock: bigint; toBlock: bigint }) => {
				const ok = toBlock - fromBlock <= cap.value
				reads.push({ from: fromBlock, to: toBlock, ok })
				if (!ok) throw new Error("Quorum not reached for getLogs: no result agreed on by a quorum")
				return []
			},
		}
		return {
			scanner,
			reads,
			setHead: (next: bigint) => {
				head = next
			},
			// biome-ignore lint/suspicious/noExplicitAny: reading the private span
			span: () => (scanner as any).span as bigint,
		}
	}

	/** One scan pass. A failed read retries once after a backoff, which the fake clock runs through. */
	async function pass(scanner: ChainScanner): Promise<"ok" | "failed"> {
		// biome-ignore lint/suspicious/noExplicitAny: driving the private loop directly
		const outcome = ((scanner as any).scan() as Promise<void>).then(
			() => "ok" as const,
			() => "failed" as const,
		)
		await vi.advanceTimersByTimeAsync(1_000)
		return outcome
	}

	beforeEach(() => {
		vi.useFakeTimers()
	})

	afterEach(() => {
		vi.useRealTimers()
	})

	it("narrows a refused range until a read succeeds, then advances without skipping a block", async () => {
		const { scanner, reads, setHead } = cappedScanner({ value: 100n })
		await pass(scanner) // the first pass only takes the cursor from the head
		const cursor = scanner.scannedTo!
		setHead(cursor + 2_000n)

		const outcomes: string[] = []
		while (scanner.scannedTo === cursor) outcomes.push(await pass(scanner))

		// 1000, 500, 250, 125 are refused; 62 fits under the cap.
		expect(outcomes).toEqual(["failed", "failed", "failed", "failed", "ok"])
		const good = reads.filter((read) => read.ok)
		expect(good).toHaveLength(1)
		expect(good[0].from).toBe(cursor + 1n)
		expect(good[0].to - good[0].from).toBe(62n)
		expect(scanner.scannedTo).toBe(good[0].to)
	})

	it("leaves the cursor where it was after a failed read", async () => {
		const { scanner, setHead } = cappedScanner({ value: 100n })
		await pass(scanner)
		const cursor = scanner.scannedTo!
		setHead(cursor + 2_000n)

		expect(await pass(scanner)).toBe("failed")
		expect(scanner.scannedTo).toBe(cursor)
	})

	it("widens back to the full range once reads succeed again", async () => {
		const cap = { value: 100n }
		const { scanner, setHead, span } = cappedScanner(cap)
		await pass(scanner)
		setHead(scanner.scannedTo! + 10_000n)
		while ((await pass(scanner)) === "failed") {}
		expect(span()).toBe(124n) // doubled from the 62 that succeeded

		cap.value = 10_000n // the endpoints recover
		const spans: bigint[] = []
		for (let i = 0; i < 5; i++) {
			await pass(scanner)
			spans.push(span())
		}
		expect(spans).toEqual([248n, 496n, 992n, 1_000n, 1_000n])
	})

	it("reads a range contiguously across failures and recoveries", async () => {
		const { scanner, reads, setHead } = cappedScanner({ value: 100n })
		await pass(scanner)
		const start = scanner.scannedTo!
		setHead(start + 600n)
		for (let i = 0; i < 40 && scanner.scannedTo! < start + 600n; i++) await pass(scanner)

		expect(scanner.scannedTo).toBe(start + 600n)
		// Every good read starts the block after the previous one ended.
		const good = reads.filter((read) => read.ok)
		good.forEach((read, i) => expect(read.from).toBe(i === 0 ? start + 1n : good[i - 1].to + 1n))
	})

	it("does not narrow for a range the RPC has not indexed yet", async () => {
		const { scanner, setHead, span } = cappedScanner({ value: 10_000n })
		await pass(scanner)
		setHead(scanner.scannedTo! + 50n)
		// biome-ignore lint/suspicious/noExplicitAny: replacing the private client with a stub
		;(scanner as any).quorumClient.getLogs = async () => {
			throw new Error("block range extends beyond current head block")
		}

		await pass(scanner)
		expect(span()).toBe(1_000n)
	})

	it("narrows to a single block, and still reads one", async () => {
		const { scanner, reads, setHead, span } = cappedScanner({ value: 0n })
		await pass(scanner)
		const cursor = scanner.scannedTo!
		setHead(cursor + 2_000n)
		for (let i = 0; i < 15 && scanner.scannedTo === cursor; i++) await pass(scanner)

		expect(scanner.scannedTo).toBe(cursor + 1n)
		const good = reads.filter((read) => read.ok)
		expect(good[0].to).toBe(good[0].from)
		expect(span()).toBe(1n)
	})
})
