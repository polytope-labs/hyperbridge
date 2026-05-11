import { describe, it, expect } from "vitest"
import { parseAbiItem } from "viem"
import { QuorumPublicClient, QuorumError, quorumThreshold } from "@/services/QuorumPublicClient"

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
