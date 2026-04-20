import { describe, it, expect } from "vitest"
import { parseAbiItem } from "viem"
import { QuorumPublicClient, QuorumError, quorumThreshold } from "@/services/QuorumPublicClient"

/**
 * Integration tests for QuorumPublicClient against public Base mainnet RPCs.
 *
 * These tests hit the public internet. They target a moving window of the most
 * recent 100 blocks so providers with different archive retention policies all
 * have the range indexed. Providers are chosen from different organisations so
 * the hostname-uniqueness check accepts them.
 */

const BASE_CHAIN_ID = 8453

// Public Base mainnet endpoints — each under a distinct hostname.
// If `BASE_MAINNET` is set in the environment (e.g. a premium endpoint in
// `.env.local`), it is added as an extra quorum member.
const BASE_PUBLIC_RPCS: string[] = [
	"https://mainnet.base.org",
	"https://base.publicnode.com",
	"https://base.llamarpc.com",
	...(process.env.BASE_MAINNET ? [process.env.BASE_MAINNET] : []),
]

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
		const client = new QuorumPublicClient(BASE_CHAIN_ID, ["https://mainnet.base.org"])
		expect(client.size).toBe(1)
		expect(client.rpcUrls).toEqual(["https://mainnet.base.org"])
	})

	it("accepts multiple distinct-hostname endpoints", () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, BASE_PUBLIC_RPCS)
		expect(client.size).toBe(BASE_PUBLIC_RPCS.length)
	})
})

describe("QuorumPublicClient.getLogs — public Base RPCs", () => {
	it("returns the same logs when every provider agrees on a recent window", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, BASE_PUBLIC_RPCS)

		// Use the quorum's own head so the window is guaranteed to be within reach
		// of `threshold` providers — avoids tip-propagation flakes where one honest
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

	it("works with a single provider (no quorum comparison)", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [BASE_PUBLIC_RPCS[0]])
		const latestBlockNumber = await client.getBlockNumber()

		const logs = await client.getLogs({
			address: USDC_ON_BASE,
			events: [TRANSFER_EVENT],
			fromBlock: latestBlockNumber - BLOCK_WINDOW,
			toBlock: latestBlockNumber,
		})

		expect(logs.length).toBeGreaterThan(0)
	}, 60_000)

	it("fails the batch when any provider is unreachable", async () => {
		// A syntactically valid URL at a guaranteed-unresolvable hostname. The
		// working providers will return logs quickly; the broken one will error
		// out and — per quorum semantics — fail the whole batch.
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [
			BASE_PUBLIC_RPCS[0],
			"https://this-host-should-never-resolve.invalid",
		])

		const singleProvider = new QuorumPublicClient(BASE_CHAIN_ID, [BASE_PUBLIC_RPCS[0]])
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
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [BASE_PUBLIC_RPCS[0], badUrl])

		const singleProvider = new QuorumPublicClient(BASE_CHAIN_ID, [BASE_PUBLIC_RPCS[0]])
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

	it("tolerates one faulty provider when N=4 (threshold=3)", async () => {
		// Three working public endpoints + one unresolvable host = N=4, threshold=3.
		// Three honest providers must agree for the batch to succeed even though
		// the fourth fails outright.
		const urls = [...BASE_PUBLIC_RPCS.slice(0, 3), "https://quorum-tolerance-test.invalid"]
		const client = new QuorumPublicClient(BASE_CHAIN_ID, urls)
		expect(client.threshold).toBe(3)

		// Pick the range from the quorum's own head — guaranteed indexed by all
		// threshold honest providers, so getLogs can't be tipped out of quorum by
		// a late-propagating block.
		const latestBlockNumber = await client.getBlockNumber()
		const logs = await client.getLogs({
			address: USDC_ON_BASE,
			events: [TRANSFER_EVENT],
			fromBlock: latestBlockNumber - BLOCK_WINDOW,
			toBlock: latestBlockNumber,
		})

		expect(logs.length).toBeGreaterThan(0)
	}, 60_000)
})

describe("QuorumPublicClient.getBlockNumber — public Base RPCs", () => {
	it("returns the highest head with quorum support (threshold-th descending)", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, BASE_PUBLIC_RPCS)
		const head = await client.getBlockNumber()
		expect(head).toBeGreaterThan(0n)

		// The quorum head must be ≤ at least `threshold` individual heads — i.e.
		// no more providers are "behind" it than the quorum tolerates.
		const individualHeads = await Promise.all(client.clients.map((c) => c.getBlockNumber()))
		const atLeast = individualHeads.filter((h) => h >= head).length
		expect(atLeast).toBeGreaterThanOrEqual(client.threshold)
	}, 60_000)

	it("fails when too many providers are unreachable (N=2 needs both)", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [
			BASE_PUBLIC_RPCS[0],
			"https://getblock-number-unreachable.invalid",
		])
		// N=2 → threshold=2, one failure already puts us below quorum.
		await expect(client.getBlockNumber()).rejects.toBeInstanceOf(QuorumError)
	}, 60_000)

	it("tolerates one faulty provider when N=4 (threshold=3)", async () => {
		const urls = [...BASE_PUBLIC_RPCS.slice(0, 3), "https://block-number-tolerance.invalid"]
		const client = new QuorumPublicClient(BASE_CHAIN_ID, urls)
		expect(client.threshold).toBe(3)

		const head = await client.getBlockNumber()
		expect(head).toBeGreaterThan(0n)
	}, 60_000)
})
