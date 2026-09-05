import { EventEmitter } from "node:events"
import type { Order } from "@hyperbridge/sdk"
import { describe, expect, it } from "vitest"
import type { EventMonitor } from "@/core/event-monitor"
import { MemoryDataStore } from "@/data/memory"
import { ActivityRecorder } from "@/data/recorder"
import type { ActivityEvent } from "@/data/types"

const USER = "0x00000000000000000000000000000000000000000000000000000000000000aa"
const USDC = "0x000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const CNGN = "0x00000000000000000000000046c85152bfe9f96829aa94755d9f915f9b10ef5f"
const REFERRER = "0x00000000000000000000000000000000000000000000000000000000000000bb"

function order(id: string): Order {
	return {
		id,
		user: USER,
		source: "EVM-8453",
		destination: "EVM-8453",
		deadline: 123n,
		nonce: 1n,
		fees: 0n,
		session: "0x",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: USDC, amount: 19_585_000_000n }],
		output: { beneficiary: USER, assets: [{ token: CNGN, amount: 26_857_230_000_000n }], call: "0x" },
	}
}

async function recorded(recorder: ActivityRecorder, count: number): Promise<ActivityEvent[]> {
	const rows: ActivityEvent[] = []
	await new Promise<void>((resolve) => {
		recorder.on("event", (row: ActivityEvent) => {
			rows.push(row)
			if (rows.length === count) resolve()
		})
	})
	return rows
}

describe("ActivityRecorder order summaries", () => {
	it("captures the order on detection and attaches it to the order's later rows", async () => {
		const store = new MemoryDataStore()
		const monitor = new EventEmitter() as unknown as EventMonitor
		const recorder = new ActivityRecorder(store.activity, undefined, {
			describeToken: async (chain, token) => {
				expect(chain).toBe("EVM-8453")
				return token === "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
					? { symbol: "USDC", decimals: 6 }
					: { symbol: "CNGN", decimals: 6 }
			},
		})
		recorder.attach(monitor)
		const rows = recorded(recorder, 2)

		monitor.emit("newOrder", { order: order("order-1"), transactionHash: "0xplaced", graffiti: REFERRER })
		// Detection awaits token lookups, so the skip is emitted after them.
		await new Promise((resolve) => setTimeout(resolve, 20))
		monitor.emit("orderSkipped", { orderId: "order-1", reason: "watch-only" })

		const [detected, skipped] = await rows
		expect(detected.type).toBe("detected")
		expect(detected.order).toEqual({
			user: "0x00000000000000000000000000000000000000aa",
			source: "EVM-8453",
			destination: "EVM-8453",
			placedTxHash: "0xplaced",
			referrer: REFERRER,
			inputs: [
				{ token: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", amount: "19585000000", symbol: "USDC", decimals: 6 },
			],
			outputs: [
				{ token: "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f", amount: "26857230000000", symbol: "CNGN", decimals: 6 },
			],
			deadline: "123",
		})
		expect(skipped.type).toBe("skipped")
		expect(skipped.order).toEqual(detected.order)
		expect((await store.activity.recent(10)).map((row) => row.order !== null)).toEqual([true, true])
	})

	it("treats a self-referral or missing graffiti as unattributed and survives a failed lookup", async () => {
		const store = new MemoryDataStore()
		const monitor = new EventEmitter() as unknown as EventMonitor
		const recorder = new ActivityRecorder(store.activity, undefined, {
			describeToken: async () => {
				throw new Error("rpc down")
			},
		})
		recorder.attach(monitor)
		const rows = recorded(recorder, 2)

		monitor.emit("newOrder", { order: order("order-2"), transactionHash: "0xa", graffiti: USER })
		monitor.emit("newOrder", { order: order("order-3"), transactionHash: "0xb" })

		const [selfReferred, untagged] = await rows
		expect(selfReferred.order?.referrer).toBeNull()
		expect(untagged.order?.referrer).toBeNull()
		expect(untagged.order?.inputs[0]).toEqual({
			token: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
			amount: "19585000000",
			symbol: null,
			decimals: null,
		})
	})

	it("records an accepted bid as a bid, then settles the order from the on-chain fill", async () => {
		const store = new MemoryDataStore()
		const monitor = new EventEmitter() as unknown as EventMonitor
		const recorder = new ActivityRecorder(store.activity)
		recorder.attach(monitor)
		const rows = recorded(recorder, 4)

		monitor.emit("newOrder", { order: order("order-4"), transactionHash: "0xplaced" })
		await new Promise((resolve) => setTimeout(resolve, 10))
		// A bid: the filler reports "filled" with a commitment, then "executed".
		monitor.emit("orderFilled", { orderId: "order-4", hash: "0xext", volumeUsd: 1, chainId: 8453, commitment: "order-4" })
		monitor.emit("orderExecuted", { orderId: "order-4", success: true, txHash: "0xext", strategy: "FXFiller", commitment: "order-4" })
		// A rival wins it on chain.
		monitor.emit("orderFillObserved", { commitment: "order-4", filler: "0xrival", chainId: 8453, txHash: "0xfill", ours: false })
		// A fill for an order this filler never saw is not the feed's business.
		monitor.emit("orderFillObserved", { commitment: "order-elsewhere", filler: "0xrival", chainId: 8453, txHash: "0xother", ours: false })
		// Our own on-chain fill of a direct (non-bid) order.
		monitor.emit("orderFillObserved", { commitment: "order-5", filler: "0xus", chainId: 8453, txHash: "0xours", ours: true })

		const [detected, bid, lost, filled] = await rows
		expect(detected.type).toBe("detected")
		expect(bid).toMatchObject({ type: "bid", orderId: "order-4", txHash: "0xext", success: true })
		expect(lost).toMatchObject({ type: "lost", orderId: "order-4", txHash: "0xfill", chainId: 8453, reason: "0xrival" })
		expect(lost.order).toEqual(detected.order)
		expect(filled).toMatchObject({ type: "filled", orderId: "order-5", txHash: "0xours", chainId: 8453 })
		expect((await store.activity.recent(10)).map((row) => row.type)).toEqual(["filled", "lost", "bid", "detected"])
		expect(await store.activity.knowsOrder("order-elsewhere")).toBe(false)
	})
})
