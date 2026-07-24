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

describe("aggregateConfirmations — tiered operator+public agreement", () => {
	const RECEIPT_BLOCK = 100n
	const HASH_A = "0xaaaa"
	const HASH_B = "0xbbbb"

	function op(head: bigint, blockHash = HASH_A, blockNumber = RECEIPT_BLOCK): ReceiptView {
		return { isOperator: true, blockHash, blockNumber, head }
	}
	function pub(head: bigint, blockHash = HASH_A, blockNumber = RECEIPT_BLOCK): ReceiptView {
		return { isOperator: false, blockHash, blockNumber, head }
	}

	// Common shape: 1 operator, 2 public required (operatorQuorum=1, requiredPublic=2).
	it("counts from the tiered head when operator + 2 public agree", () => {
		// heads: op 120, public 118 and 115. Tiered head = min(op[0]=120, pub[1]=115) = 115 → 115-100+1 = 16.
		const views = [op(120n), pub(118n), pub(115n)]
		expect(aggregateConfirmations(views, 1, 2)).toBe(16n)
	})

	it("returns null when the operator is not among the receipt-holders", () => {
		// Two stale public endpoints agree, but no operator does — the critical
		// reorg case: a minority serving a pre-reorg receipt must not reach quorum.
		const views = [pub(120n), pub(118n)]
		expect(aggregateConfirmations(views, 1, 2)).toBeNull()
	})

	it("returns null when fewer than the required public witnesses agree", () => {
		// Operator + only 1 public hold the receipt; the public floor is 2.
		const views = [op(120n), pub(118n)]
		expect(aggregateConfirmations(views, 1, 2)).toBeNull()
	})

	it("ignores a divergent minority and counts the agreeing tiered group", () => {
		// One public serves a different inclusion (HASH_B); the operator + 2 public
		// agreeing on HASH_A still form the quorum.
		const views = [op(120n), pub(118n), pub(117n), pub(999n, HASH_B, 90n)]
		expect(aggregateConfirmations(views, 1, 2)).toBe(18n) // min(120, pub[1]=117) = 117 → 117-100+1
	})

	it("floors at zero when the tiered head trails the inclusion block", () => {
		const views = [op(99n), pub(98n), pub(99n)]
		expect(aggregateConfirmations(views, 1, 2)).toBe(0n)
	})

	it("operator-only quorum (no public configured) needs just the operator BFT quorum", () => {
		// requiredPublic=0: two operators agreeing is enough (e.g. a non-registry chain).
		const views = [op(105n), op(104n)]
		expect(aggregateConfirmations(views, 2, 0)).toBe(5n) // 2nd op head = 104 → 104-100+1
	})

	it("empty views never reach quorum", () => {
		expect(aggregateConfirmations([], 1, 2)).toBeNull()
	})
})

describe("QuorumPublicClient — tiered failure handling (stubbed clients)", () => {
	// operatorCount leading URLs are the operator's; the rest public. Stubs are
	// swapped in post-construction (URLs are .invalid, never contacted).
	function makeClient(operatorCount: number, publicCount: number): QuorumPublicClient {
		const urls = [
			...Array.from({ length: operatorCount }, (_, i) => `https://op-${i}.invalid`),
			...Array.from({ length: publicCount }, (_, i) => `https://pub-${i}.invalid`),
		]
		return new QuorumPublicClient(BASE_CHAIN_ID, urls, operatorCount)
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

	it("reports the tier split on the client", () => {
		const c = makeClient(1, 4)
		expect(c.operatorCount).toBe(1)
		expect(c.publicCount).toBe(4)
		expect(c.operatorQuorum).toBe(1)
		expect(c.requiredPublic).toBe(2)
	})

	it("getBlockNumber fails when the operator does not respond (operator failure is intolerable)", async () => {
		const c = makeClient(1, 2)
		c.clients[0] = errHead("operator down")
		c.clients[1] = okHead(100n)
		c.clients[2] = okHead(100n)
		await expect(c.getBlockNumber()).rejects.toThrow(/Quorum not reached/)
	})

	it("getBlockNumber fails when fewer than 2 public corroborate", async () => {
		const c = makeClient(1, 2)
		c.clients[0] = okHead(100n)
		c.clients[1] = okHead(100n)
		c.clients[2] = errHead("public down")
		await expect(c.getBlockNumber()).rejects.toThrow(/Quorum not reached/)
	})

	it("getBlockNumber returns the tiered head when operator + 2 public respond", async () => {
		const c = makeClient(1, 2)
		c.clients[0] = okHead(120n) // operator
		c.clients[1] = okHead(118n)
		c.clients[2] = okHead(115n)
		// min(op head 120, 2nd public head 115) = 115.
		await expect(c.getBlockNumber()).resolves.toBe(115n)
	})

	it("a public endpoint failing is tolerated when 2 others still corroborate", async () => {
		const c = makeClient(1, 4)
		c.clients[0] = okHead(120n)
		c.clients[1] = okHead(118n)
		c.clients[2] = okHead(117n)
		c.clients[3] = errHead("throttled")
		c.clients[4] = errHead("down")
		await expect(c.getBlockNumber()).resolves.toBe(117n) // min(120, pub[1]=117)
	})

	it("confirmations: a stale public minority cannot reach quorum after a reorg", async () => {
		const c = makeClient(1, 4)
		// Operator and two publics no longer see the tx (reorged out) — not-found.
		c.clients[0] = notFoundClient(200n)
		c.clients[1] = notFoundClient(200n)
		c.clients[2] = notFoundClient(200n)
		// Two stale publics still serve the pre-reorg receipt.
		c.clients[3] = receiptClient(200n, "0xdead", 100n)
		c.clients[4] = receiptClient(200n, "0xdead", 100n)
		// No operator holds the receipt → operator quorum unmet → throws.
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).rejects.toThrow(/Quorum not reached/)
	})

	it("confirmations: operator not-found blocks confirmation even if all public agree", async () => {
		const c = makeClient(1, 4)
		c.clients[0] = notFoundClient(200n) // operator hasn't/doesn't see it
		c.clients[1] = receiptClient(200n, "0xabc", 100n)
		c.clients[2] = receiptClient(200n, "0xabc", 100n)
		c.clients[3] = receiptClient(200n, "0xabc", 100n)
		c.clients[4] = receiptClient(200n, "0xabc", 100n)
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).rejects.toThrow(/Quorum not reached/)
	})

	it("confirmations: succeeds when operator + 2 public agree on the inclusion", async () => {
		const c = makeClient(1, 4)
		c.clients[0] = receiptClient(120n, "0xabc", 100n) // operator
		c.clients[1] = receiptClient(118n, "0xabc", 100n)
		c.clients[2] = receiptClient(115n, "0xabc", 100n)
		c.clients[3] = notFoundClient(120n)
		c.clients[4] = errHead("down")
		// tiered head = min(op 120, 2nd public 115) = 115 → 115-100+1 = 16.
		await expect(c.getTransactionConfirmations({ hash: "0x1" as any })).resolves.toBe(16n)
	})
})
