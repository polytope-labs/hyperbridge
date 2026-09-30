import { describe, expect, it, vi } from "vitest"
import { backfillOrderSummaries } from "@/data/backfill"
import { MemoryDataStore } from "@/data/memory"
import type { ActivityEvent } from "@/data/types"
import { getLogger } from "@/services/Logger"

const COMMITMENT = "0x7da92018c62aa9f3dbeef16e0033618bfd6614749b1a3c7e84ff2a0385a079b8"
const HYPERFX_TAG = "0x4879706572465800000000000000000000000000000000000000000000000000"

function indexerResponse(nodes: unknown[]) {
	return new Response(JSON.stringify({ data: { iOrderV3s: { nodes } } }), {
		status: 200,
		headers: { "content-type": "application/json" },
	})
}

describe("activity backfill from the indexer", () => {
	it("attaches an indexed order to every row that lacked one and reports them", async () => {
		const store = new MemoryDataStore()
		await store.activity.record({ type: "detected", orderId: COMMITMENT })
		await store.activity.record({ type: "filled", orderId: COMMITMENT, txHash: "0xfill", chainId: 8453 })
		await store.activity.record({ type: "detected", orderId: "0xunknown" })
		const fetchImpl = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
			const { variables } = JSON.parse(String(init?.body)) as { variables: { commitment: string } }
			if (variables.commitment !== COMMITMENT) return indexerResponse([])
			return indexerResponse([
				{
					user: "0xeb9c8aa8048a21f5df9a39e37291bdc45da10e8a",
					sourceChain: "EVM-8453",
					destChain: "EVM-8453",
					deadline: "50903149",
					referrer: HYPERFX_TAG,
					transactionHash: "0x2968",
					inputAssets: {
						nodes: [{ token: "0x000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913", amount: "19585032933", index: 0 }],
					},
					outputAssets: {
						nodes: [{ token: "0x00000000000000000000000046c85152bfe9f96829aa94755d9f915f9b10ef5f", amount: "26857229849348", index: 0 }],
					},
				},
			])
		})
		const updated: ActivityEvent[] = []
		const totals = await backfillOrderSummaries({
			store: store.activity,
			indexerUrl: "https://indexer.test/",
			describeToken: async (_chain, token) =>
				token.startsWith("0x8335") ? { symbol: "USDC", decimals: 6 } : { symbol: "CNGN", decimals: 6 },
			onUpdated: (rows) => updated.push(...rows),
			logger: getLogger("test"),
			fetchImpl: fetchImpl as unknown as typeof fetch,
		})

		expect(totals).toEqual({ orders: 1, rows: 2, settled: 0 })
		expect(fetchImpl).toHaveBeenCalledTimes(2)
		expect(updated.map((row) => row.type).sort()).toEqual(["detected", "filled"])
		const summary = updated[0].order
		expect(summary).toEqual({
			user: "0xeb9c8aa8048a21f5df9a39e37291bdc45da10e8a",
			source: "EVM-8453",
			destination: "EVM-8453",
			placedTxHash: "0x2968",
			referrer: HYPERFX_TAG,
			inputs: [{ token: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", amount: "19585032933", symbol: "USDC", decimals: 6 }],
			outputs: [{ token: "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f", amount: "26857229849348", symbol: "CNGN", decimals: 6 }],
			deadline: "50903149",
		})
		// The unknown order stays without details; a second run has nothing left for the known one.
		expect(await store.activity.orderIdsMissingSummary()).toEqual(["0xunknown"])
	})

	it("survives an indexer failure and touches nothing", async () => {
		const store = new MemoryDataStore()
		await store.activity.record({ type: "detected", orderId: COMMITMENT })
		const totals = await backfillOrderSummaries({
			store: store.activity,
			indexerUrl: "https://indexer.test/",
			logger: getLogger("test"),
			fetchImpl: (async () => new Response("boom", { status: 502 })) as unknown as typeof fetch,
		})
		expect(totals).toEqual({ orders: 0, rows: 0, settled: 0 })
		expect((await store.activity.recent(1))[0].order).toBeNull()
	})

	it("settles a bid from the indexer's fill record and retypes a legacy bid-time fill", async () => {
		const store = new MemoryDataStore()
		// Legacy shape: a bid recorded as "filled" with the bid's volume, and the extrinsic as its hash.
		await store.activity.record({ type: "detected", orderId: COMMITMENT })
		await store.activity.record({ type: "filled", orderId: COMMITMENT, txHash: "0xext", volumeUsd: 1, chainId: 8453 })
		// A bid still open on chain: nothing to settle yet.
		await store.activity.record({ type: "bid", orderId: "0xopen", txHash: "0xext2" })
		const fetchImpl = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
			const { variables } = JSON.parse(String(init?.body)) as { variables: { commitment: string } }
			const base = {
				user: "0xeb9c8aa8048a21f5df9a39e37291bdc45da10e8a",
				sourceChain: "EVM-8453",
				destChain: "EVM-8453",
				deadline: "1",
				referrer: null,
				transactionHash: "0xplaced",
				inputAssets: { nodes: [] },
				outputAssets: { nodes: [] },
			}
			if (variables.commitment === "0xopen") return indexerResponse([{ ...base, statusMetadata: { nodes: [{ status: "PLACED", chain: "8453", transactionHash: "0xplaced", filler: null }] } }])
			return indexerResponse([
				{
					...base,
					statusMetadata: {
						nodes: [
							{ status: "PLACED", chain: "8453", transactionHash: "0xplaced", filler: null },
							{ status: "FILLED", chain: "8453", transactionHash: "0xfilltx", filler: "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" },
						],
					},
				},
			])
		})
		const updated: ActivityEvent[] = []
		const totals = await backfillOrderSummaries({
			store: store.activity,
			indexerUrl: "https://indexer.test/",
			fillerAddress: "0x21426D68a9E5Df153FE75cE0fEd20173EBcb80eF",
			onUpdated: (rows) => updated.push(...rows),
			logger: getLogger("test"),
			fetchImpl: fetchImpl as unknown as typeof fetch,
		})

		expect(totals.settled).toBe(1)
		const rows = await store.activity.recent(10)
		const ours = rows.filter((row) => row.orderId === COMMITMENT).map((row) => row.type)
		expect(ours).toEqual(["lost", "bid", "detected"])
		const lost = rows.find((row) => row.type === "lost")
		expect(lost).toMatchObject({ txHash: "0xfilltx", chainId: 8453, reason: "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" })
		// Pass 1 also reported the summary attachments; pass 2 reported the retyped bid and the loss.
		expect(updated.filter((row) => row.type === "lost")).toHaveLength(1)
		expect(updated.some((row) => row.type === "bid" && row.orderId === COMMITMENT)).toBe(true)
		// Nothing left to settle for it; the open bid still waits.
		expect(await store.activity.unsettledOrders()).toEqual(["0xopen"])
	})
})
