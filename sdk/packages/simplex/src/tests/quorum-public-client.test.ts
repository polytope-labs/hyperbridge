import { describe, it, expect } from "vitest"
import { parseAbiItem } from "viem"
import {
	QuorumPublicClient,
	QuorumError,
	quorumThreshold,
	aggregateConfirmations,
	type ReceiptView,
} from "@/services/QuorumPublicClient"

/**
 * Integration tests for QuorumPublicClient against a two-RPC quorum on Base mainnet.
 *
 * The quorum is formed from the official public endpoint (`mainnet.base.org`) and
 * a second endpoint supplied by the operator via the `BASE_MAINNET` env var —
 * typically a premium node in `.env.local`. With N=2 the threshold is 2, so both
 * providers must succeed and agree for every batch; this is the smallest useful
 * quorum and the one operators most commonly run.
 *
 * Tests that need the real network are skipped if `BASE_MAINNET` is unset so the
 * suite still runs (constructor-only coverage) in environments without credentials.
 */

const BASE_CHAIN_ID = 8453

const OFFICIAL_BASE_RPC = "https://mainnet.base.org"
const ENV_BASE_RPC = process.env.BASE_MAINNET

const NETWORK_QUORUM_RPCS: string[] = ENV_BASE_RPC ? [OFFICIAL_BASE_RPC, ENV_BASE_RPC] : []

const describeIfNetwork = NETWORK_QUORUM_RPCS.length === 2 ? describe : describe.skip

// Base mainnet USDC. Any full node should serve recent logs for this contract.
const USDC_ON_BASE = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"

const TRANSFER_EVENT = parseAbiItem(
	"event Transfer(address indexed from, address indexed to, uint256 value)",
)

// Small enough that a window of Base USDC transfers stays under the official
// gateway's response-size cap ("backend response too large" at 100 blocks
// during busy periods), large enough to always contain at least one transfer.
const BLOCK_WINDOW = 10n

describe("quorumThreshold", () => {
	it.each<[number, number]>([
		[1, 1],
		[2, 2],
		[3, 3],
		[4, 3],
		[5, 4],
		[6, 5],
		[7, 5],
		[9, 7],
		[10, 7],
	])("N=%i → threshold=%i (floor(2N/3) + 1)", (n, expected) => {
		expect(quorumThreshold(n)).toBe(expected)
	})
})

describe("QuorumPublicClient — constructor validation", () => {
	it("rejects two URLs that share a hostname", () => {
		expect(
			() =>
				new QuorumPublicClient(BASE_CHAIN_ID, [
					"https://mainnet.base.org/one",
					"https://mainnet.base.org/two",
				]),
		).toThrow(/different domains/)
	})

	it("rejects an empty URL list", () => {
		expect(() => new QuorumPublicClient(BASE_CHAIN_ID, [])).toThrow(/at least one URL/)
	})

	it("rejects malformed URLs", () => {
		expect(() => new QuorumPublicClient(BASE_CHAIN_ID, ["not-a-url"])).toThrow(/Invalid RPC URL/)
	})

	it("accepts a single endpoint and reports size 1", () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [OFFICIAL_BASE_RPC])
		expect(client.size).toBe(1)
		expect(client.threshold).toBe(1)
		expect(client.rpcUrls).toEqual([OFFICIAL_BASE_RPC])
	})
})

describeIfNetwork("QuorumPublicClient.getLogs — N=2 Base RPCs", () => {
	it("both providers agree on a recent window", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, NETWORK_QUORUM_RPCS)
		expect(client.threshold).toBe(2)

		// Use the quorum's own head so the window is guaranteed to be within reach
		// of both providers — avoids tip-propagation flakes where one honest
		// provider hasn't indexed up to another's reported head yet.
		const latestBlockNumber = await client.getBlockNumber()
		const fromBlock = latestBlockNumber - BLOCK_WINDOW
		const toBlock = latestBlockNumber

		const logs = await client.getLogs({
			address: USDC_ON_BASE,
			events: [TRANSFER_EVENT],
			fromBlock,
			toBlock,
		})

		expect(Array.isArray(logs)).toBe(true)
		expect(logs.length).toBeGreaterThan(0)
		for (const log of logs) {
			expect(log.address.toLowerCase()).toBe(USDC_ON_BASE.toLowerCase())
			expect(log.blockNumber).not.toBeNull()
			if (log.blockNumber !== null) {
				expect(log.blockNumber >= fromBlock).toBe(true)
				expect(log.blockNumber <= toBlock).toBe(true)
			}
		}
	}, 60_000)

	it("fails the batch when one of the two providers is unreachable", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [
			OFFICIAL_BASE_RPC,
			"https://this-host-should-never-resolve.invalid",
		])

		const singleProvider = new QuorumPublicClient(BASE_CHAIN_ID, [OFFICIAL_BASE_RPC])
		const latestBlockNumber = await singleProvider.getBlockNumber()

		await expect(
			client.getLogs({
				address: USDC_ON_BASE,
				events: [TRANSFER_EVENT],
				fromBlock: latestBlockNumber - BLOCK_WINDOW,
				toBlock: latestBlockNumber,
			}),
		).rejects.toBeInstanceOf(QuorumError)
	}, 60_000)

	it("surfaces the offending provider URL in the QuorumError", async () => {
		const badUrl = "https://another-unresolvable-host.invalid"
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [OFFICIAL_BASE_RPC, badUrl])

		const singleProvider = new QuorumPublicClient(BASE_CHAIN_ID, [OFFICIAL_BASE_RPC])
		const latestBlockNumber = await singleProvider.getBlockNumber()

		let caught: unknown
		try {
			await client.getLogs({
				address: USDC_ON_BASE,
				events: [TRANSFER_EVENT],
				fromBlock: latestBlockNumber - BLOCK_WINDOW,
				toBlock: latestBlockNumber,
			})
		} catch (error) {
			caught = error
		}

		expect(caught).toBeInstanceOf(QuorumError)
		expect((caught as QuorumError).message).toContain(badUrl)
	}, 60_000)
})

describeIfNetwork("QuorumPublicClient.getBlockNumber — N=2 Base RPCs", () => {
	it("returns a head ≤ both providers' individual heads", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, NETWORK_QUORUM_RPCS)
		expect(client.threshold).toBe(2)

		const head = await client.getBlockNumber()
		expect(head).toBeGreaterThan(0n)

		const individualHeads = await Promise.all(client.clients.map((c) => c.getBlockNumber()))
		for (const h of individualHeads) {
			expect(head <= h).toBe(true)
		}
	}, 60_000)

	it("fails when one of the two providers is unreachable", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [
			OFFICIAL_BASE_RPC,
			"https://getblock-number-unreachable.invalid",
		])
		await expect(client.getBlockNumber()).rejects.toBeInstanceOf(QuorumError)
	}, 60_000)
})

describe("aggregateConfirmations — BFT agreement on an inclusion", () => {
	const RECEIPT_BLOCK = 100n
	const HASH_A = "0xaaaa"
	const HASH_B = "0xbbbb"

	function view(head: bigint, blockHash = HASH_A, blockNumber = RECEIPT_BLOCK): ReceiptView {
		return { blockHash, blockNumber, head }
	}

	it("counts from the quorum head when a quorum agrees on the inclusion", () => {
		// heads 120, 118, 115 with quorum=2: the 2nd-highest head (118) bounds the
		// depth → 118-100+1 = 19.
		const views = [view(120n), view(118n), view(115n)]
		expect(aggregateConfirmations(views, 2)).toBe(19n)
	})

	it("returns null when fewer than the quorum hold the receipt", () => {
		const views = [view(120n)]
		expect(aggregateConfirmations(views, 2)).toBeNull()
	})

	it("ignores a divergent minority and counts the agreeing group", () => {
		// One endpoint serves a different inclusion (HASH_B, e.g. pre-reorg); the
		// two agreeing on HASH_A still form the quorum.
		const views = [view(120n), view(117n), view(999n, HASH_B, 90n)]
		expect(aggregateConfirmations(views, 2)).toBe(18n) // 2nd head = 117 → 117-100+1
	})

	it("a reorged-out minority cannot reach quorum on its own", () => {
		// Only stale endpoints still serve the pre-reorg receipt; below quorum.
		const views = [view(200n, "0xdead", 100n)]
		expect(aggregateConfirmations(views, 2)).toBeNull()
	})

	it("floors at zero when the quorum head trails the inclusion block", () => {
		const views = [view(99n), view(98n)]
		expect(aggregateConfirmations(views, 2)).toBe(0n)
	})

	it("empty views never reach quorum", () => {
		expect(aggregateConfirmations([], 1)).toBeNull()
	})
})

describe("QuorumPublicClient — failure handling (stubbed clients)", () => {
	// Stubs are swapped in post-construction (URLs are .invalid, never contacted).
	function makeClient(size: number): QuorumPublicClient {
		const urls = Array.from({ length: size }, (_, i) => `https://rpc-${i}.invalid`)
		return new QuorumPublicClient(BASE_CHAIN_ID, urls)
	}
	const okHead = (head: bigint) => ({ getBlockNumber: async () => head }) as any
	const errHead = (message: string) =>
		({
			getBlockNumber: async () => {
				throw new Error(message)
			},
		}) as any
	const receiptClient = (head: bigint, blockHash: string, blockNumber: bigint) =>
		({
			getBlockNumber: async () => head,
			getTransactionReceipt: async () => ({ blockHash, blockNumber }),
		}) as any
	const notFoundClient = (head: bigint) =>
		({
			getBlockNumber: async () => head,
			getTransactionReceipt: async () => {
				const e = new Error("Transaction receipt could not be found")
				e.name = "TransactionReceiptNotFoundError"
				throw e
			},
		}) as any

	it("reports the BFT threshold on the client", () => {
		const c = makeClient(4)
		expect(c.size).toBe(4)
		expect(c.threshold).toBe(3)
	})

	it("getBlockNumber fails when fewer than the threshold respond", async () => {
		const c = makeClient(3) // threshold 3
		c.clients[0] = okHead(100n)
		c.clients[1] = okHead(100n)
		c.clients[2] = errHead("down")
		await expect(c.getBlockNumber()).rejects.toThrow(/Quorum not reached/)
	})

	it("getBlockNumber returns the threshold-th highest head", async () => {
		const c = makeClient(3)
		c.clients[0] = okHead(120n)
		c.clients[1] = okHead(118n)
		c.clients[2] = okHead(115n)
		// All three must back the head: the 3rd-highest (115) is the highest block
		// every quorum member has indexed.
		await expect(c.getBlockNumber()).resolves.toBe(115n)
	})

	it("a failing endpoint is tolerated when a quorum still responds", async () => {
		const c = makeClient(4) // threshold 3
		c.clients[0] = okHead(120n)
		c.clients[1] = okHead(118n)
		c.clients[2] = okHead(117n)
		c.clients[3] = errHead("throttled")
		await expect(c.getBlockNumber()).resolves.toBe(117n)
	})

	it("confirmations: a stale minority cannot reach quorum after a reorg", async () => {
		const c = makeClient(5) // threshold 4
		// Three endpoints no longer see the tx (reorged out) — not-found.
		c.clients[0] = notFoundClient(200n)
		c.clients[1] = notFoundClient(200n)
		c.clients[2] = notFoundClient(200n)
		// Two stale endpoints still serve the pre-reorg receipt.
		c.clients[3] = receiptClient(200n, "0xdead", 100n)
		c.clients[4] = receiptClient(200n, "0xdead", 100n)
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).rejects.toThrow(/Quorum not reached/)
	})

	it("confirmations: not-found votes block confirmation below the threshold", async () => {
		const c = makeClient(4) // threshold 3
		c.clients[0] = notFoundClient(200n)
		c.clients[1] = notFoundClient(200n)
		c.clients[2] = receiptClient(200n, "0xabc", 100n)
		c.clients[3] = receiptClient(200n, "0xabc", 100n)
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).rejects.toThrow(/Quorum not reached/)
	})

	it("confirmations: succeeds when a quorum agrees on the inclusion", async () => {
		const c = makeClient(4) // threshold 3
		c.clients[0] = receiptClient(120n, "0xabc", 100n)
		c.clients[1] = receiptClient(118n, "0xabc", 100n)
		c.clients[2] = receiptClient(115n, "0xabc", 100n)
		c.clients[3] = errHead("down")
		// quorum head = 3rd-highest agreeing head = 115 → 115-100+1 = 16.
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).resolves.toBe(16n)
	})

	it("confirmations: a coexisting not-found vote neither vetoes the quorum nor leaks its head", async () => {
		const c = makeClient(4) // threshold 3
		c.clients[0] = receiptClient(120n, "0xabc", 100n)
		c.clients[1] = receiptClient(118n, "0xabc", 100n)
		c.clients[2] = receiptClient(115n, "0xabc", 100n)
		// A responsive endpoint that hasn't indexed the tx yet — the normal state
		// during every confirmation poll. Its (higher) head must not enter the
		// depth math: quorum head = 3rd-highest RECEIPT-HOLDER head = 115 → 16,
		// not 19 (which would leak the not-found head 200 into the sort).
		c.clients[3] = notFoundClient(200n)
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).resolves.toBe(16n)
	})

	it("getLogs returns the majority batch over a divergent minority", async () => {
		const c = makeClient(4) // threshold 3
		const logA = { address: "0xa", blockHash: "0xb", blockNumber: 1n, data: "0x", logIndex: 0, removed: false, topics: [], transactionHash: "0xt", transactionIndex: 0 }
		const withLogs = (logs: unknown[]) => ({ getLogs: async () => logs }) as any
		c.clients[0] = withLogs([logA])
		c.clients[1] = withLogs([logA])
		c.clients[2] = withLogs([logA])
		c.clients[3] = withLogs([]) // lagging or pruned node
		await expect(c.getLogs({} as any)).resolves.toEqual([logA])
	})

	it("getLogs fails when no group reaches the threshold", async () => {
		const c = makeClient(3) // threshold 3
		const logA = { address: "0xa", blockHash: "0xb", blockNumber: 1n, data: "0x", logIndex: 0, removed: false, topics: [], transactionHash: "0xt", transactionIndex: 0 }
		const withLogs = (logs: unknown[]) => ({ getLogs: async () => logs }) as any
		c.clients[0] = withLogs([logA])
		c.clients[1] = withLogs([logA])
		c.clients[2] = withLogs([]) // divergent — 2/3 agree, below threshold 3
		await expect(c.getLogs({} as any)).rejects.toThrow(/Quorum not reached/)
	})

	// ── Early exit: reads resolve the moment the quorum is met; a hung provider
	// (30s timeout × 3 retries in production) must never stall the hot path once
	// its vote can no longer change the outcome. ──

	/** A provider whose requests never settle. */
	const hungClient = () =>
		({
			getBlockNumber: () => new Promise(() => {}),
			getTransactionReceipt: () => new Promise(() => {}),
			getLogs: () => new Promise(() => {}),
		}) as any
	/** Fails crisply when early exit doesn't trigger, instead of a suite timeout. */
	const within = <T>(promise: Promise<T>, ms = 2000): Promise<T> =>
		Promise.race([
			promise,
			new Promise<T>((_, reject) =>
				setTimeout(() => reject(new Error("no early exit: call blocked on the hung provider")), ms),
			),
		])

	it("getBlockNumber resolves once the quorum is met, ignoring hung providers", async () => {
		const c = makeClient(5) // threshold 4
		c.clients[0] = okHead(120n)
		c.clients[1] = okHead(119n)
		c.clients[2] = okHead(118n)
		c.clients[3] = okHead(117n)
		c.clients[4] = hungClient()
		// 4th-highest head among the responders = 117.
		await expect(within(c.getBlockNumber())).resolves.toBe(117n)
	})

	it("confirmations resolve on a positive quorum without waiting for stragglers", async () => {
		const c = makeClient(5) // threshold 4
		c.clients[0] = receiptClient(120n, "0xabc", 100n)
		c.clients[1] = receiptClient(118n, "0xabc", 100n)
		c.clients[2] = receiptClient(116n, "0xabc", 100n)
		c.clients[3] = receiptClient(115n, "0xabc", 100n)
		c.clients[4] = hungClient()
		// quorum head = 4th-highest agreeing head = 115 → 115-100+1 = 16.
		await expect(within(c.getTransactionConfirmations({ hash: "0x1" as any }))).resolves.toBe(16n)
	})

	it("getLogs returns the agreed result while stragglers are still pending", async () => {
		const c = makeClient(5) // threshold 4
		const emptyLogs = () => ({ getLogs: async () => [] }) as any
		c.clients[0] = emptyLogs()
		c.clients[1] = emptyLogs()
		c.clients[2] = emptyLogs()
		c.clients[3] = emptyLogs()
		c.clients[4] = hungClient()
		await expect(within(c.getLogs({} as any))).resolves.toEqual([])
	})
})
