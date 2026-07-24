import { describe, it, expect } from "vitest"
import { parseAbiItem } from "viem"
import {
	QuorumPublicClient,
	QuorumError,
	quorumThreshold,
	aggregateConfirmations,
	type ProviderReceiptView,
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

const BLOCK_WINDOW = 100n

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

describeIfNetwork("QuorumPublicClient.getLogs — N=2 public + env Base RPCs", () => {
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

describeIfNetwork("QuorumPublicClient.getBlockNumber — N=2 public + env Base RPCs", () => {
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

describe("aggregateConfirmations — BFT receipt agreement", () => {
	const RECEIPT_BLOCK = 100n
	const HASH_A = "0xaaaa"
	const HASH_B = "0xbbbb"

	function view(head: bigint, blockHash = HASH_A, blockNumber = RECEIPT_BLOCK): ProviderReceiptView {
		return { blockHash, blockNumber, head }
	}

	it("counts confirmations from the threshold-th highest head of the agreeing group", () => {
		// 5 providers all agree on the receipt; heads diverge. threshold=4 →
		// the 4th-highest head (115) backs the count: 115 - 100 + 1 = 16.
		const views = [view(120n), view(118n), view(117n), view(115n), view(110n)]
		expect(aggregateConfirmations(views, 4)).toBe(16n)
	})

	it("ignores a minority provider reporting a different inclusion block", () => {
		// One provider serves a reorged/fabricated receipt. The agreeing four still
		// form a quorum; the liar's head cannot influence the count.
		const views = [view(120n), view(118n), view(117n), view(115n), view(999n, HASH_B, 90n)]
		expect(aggregateConfirmations(views, 4)).toBe(16n)
	})

	it("returns null when no receipt identity reaches the threshold", () => {
		const views = [view(120n), view(118n), view(999n, HASH_B), view(998n, HASH_B)]
		expect(aggregateConfirmations(views, 3)).toBeNull()
	})

	it("returns null when there are no successful views", () => {
		expect(aggregateConfirmations([], 1)).toBeNull()
	})

	it("floors at zero when the quorum head trails the inclusion block", () => {
		// Providers agree on the receipt but their heads are behind it (possible
		// mid-reorg or load-balanced lagging replicas). Never negative.
		const views = [view(99n), view(98n), view(99n)]
		expect(aggregateConfirmations(views, 3)).toBe(0n)
	})

	it("single provider degrades to plain confirmation counting", () => {
		expect(aggregateConfirmations([view(100n)], 1)).toBe(1n)
		expect(aggregateConfirmations([view(105n)], 1)).toBe(6n)
	})
})

describe("failure policy — 429s are skipped, other errors pause the endpoint", () => {
	// The first `operatorCount` URLs are the operator's own; they set the floor
	// the per-call threshold can never drop below. Client stubs are swapped in
	// post-construction (URLs are .invalid and never contacted).
	function makeClient(urls: string[], operatorCount: number): QuorumPublicClient {
		return new QuorumPublicClient(BASE_CHAIN_ID, urls, operatorCount)
	}
	const THREE = ["https://operator.invalid", "https://registry-a.invalid", "https://registry-b.invalid"]
	const ok = (head: bigint) => ({ getBlockNumber: async () => head }) as any
	const failWith = (message: string) =>
		({
			getBlockNumber: async () => {
				throw new Error(message)
			},
		}) as any

	it("excludes a 429ing endpoint from the call without pausing it", async () => {
		const client = makeClient(THREE, 1)
		client.clients[0] = ok(100n)
		client.clients[1] = ok(100n)
		client.clients[2] = failWith("429 Too Many Requests")

		// 2 responders, threshold max(q(2)=2, q(1)=1) = 2 → succeeds immediately.
		await expect(client.getBlockNumber()).resolves.toBe(100n)
		// Rate limiting is call-local noise — the endpoint is not paused.
		expect((client as any).pausedUntil[2]).toBe(0)
	})

	it("pauses an endpoint that errors with anything other than a 429", async () => {
		const client = makeClient(THREE, 1)
		client.clients[0] = ok(100n)
		client.clients[1] = ok(100n)
		client.clients[2] = failWith("connect ECONNREFUSED")

		await expect(client.getBlockNumber()).resolves.toBe(100n)
		expect((client as any).pausedUntil[2]).toBeGreaterThan(Date.now())

		// While paused the endpoint is not queried at all: even if it would now
		// answer (divergently), the result is unchanged.
		let queried = false
		client.clients[2] = {
			getBlockNumber: async () => {
				queried = true
				return 999999n
			},
		} as any
		await expect(client.getBlockNumber()).resolves.toBe(100n)
		expect(queried).toBe(false)
	})

	it("the threshold never drops below the operator set's own bound", async () => {
		// Three operator endpoints (floor q(3) = 3) + one registry endpoint.
		const client = makeClient(
			[
				"https://op-a.invalid",
				"https://op-b.invalid",
				"https://op-c.invalid",
				"https://registry-a.invalid",
			],
			3,
		)
		client.clients[0] = ok(100n)
		client.clients[1] = failWith("429 Too Many Requests")
		client.clients[2] = failWith("429 Too Many Requests")
		client.clients[3] = ok(100n)

		// Only 2 responders agree, but the operator floor demands 3 — an outage
		// of the operator's own endpoints must not quietly weaken the quorum.
		await expect(client.getBlockNumber()).rejects.toThrow(/Quorum not reached/)
	})

	it("a divergent responder is counted, not excluded — and gets outvoted", async () => {
		const client = makeClient(THREE, 1)
		client.clients[0] = ok(100n)
		client.clients[1] = ok(100n)
		client.clients[2] = ok(999999n) // lying about the head — but responding

		// 3 responders → threshold 3; the BFT head selection discounts the outlier:
		// threshold-th highest of [999999, 100, 100] is 100.
		await expect(client.getBlockNumber()).resolves.toBe(100n)
		expect((client as any).pausedUntil[2]).toBe(0)
	})
})
