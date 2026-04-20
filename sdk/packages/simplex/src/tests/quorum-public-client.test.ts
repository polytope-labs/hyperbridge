import { describe, it, expect } from "vitest"
import { parseAbiItem } from "viem"
import { QuorumPublicClient, QuorumError } from "@/services/QuorumPublicClient"

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
})

describe("QuorumPublicClient.getBlockNumber — public Base RPCs", () => {
	it("returns the lowest head across all providers", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, BASE_PUBLIC_RPCS)
		const head = await client.getBlockNumber()
		expect(head).toBeGreaterThan(0n)

		// Upper bound: the quorum head must not exceed any individual provider's head.
		const individualHeads = await Promise.all(client.clients.map((c) => c.getBlockNumber()))
		for (const h of individualHeads) {
			expect(head <= h).toBe(true)
		}
	}, 60_000)

	it("fails when any provider is unreachable", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [
			BASE_PUBLIC_RPCS[0],
			"https://getblock-number-unreachable.invalid",
		])
		await expect(client.getBlockNumber()).rejects.toBeTruthy()
	}, 60_000)
})
