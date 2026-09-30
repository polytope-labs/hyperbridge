import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { OrderStatus, type IndexerQueryClient } from "@/types"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const commitment = "0x1234" as const

function response(statuses: OrderStatus[]) {
	return {
		orderPlaceds: {
			nodes: [
				{
					id: commitment,
					status: OrderStatus.PLACED,
					deadline: "100",
					nonce: "1",
					fees: "0",
					inputAmounts: [],
					outputAmounts: [],
					blockNumber: "1",
					blockTimestamp: "1000",
					createdAt: "2026-09-12T00:00:00Z",
					statusMetadata: {
						nodes: statuses.map((status, index) => ({
							status,
							blockNumber: String(index + 1),
							timestamp: String(1000 + index),
							transactionHash: "0x1234",
						})),
					},
				},
			],
		},
	}
}

describe("IntentGateway.orderStatusStream cancellation", () => {
	beforeEach(() => vi.useFakeTimers())
	afterEach(() => vi.useRealTimers())

	it.each(["later timestamp", "same block"])(
		"prefers a refund over cancellation metadata with %s",
		async (timing) => {
			const data = response([OrderStatus.PLACED, OrderStatus.REFUNDED, OrderStatus.CANCELLED])
			if (timing === "same block") {
				for (const item of data.orderPlaceds.nodes[0].statusMetadata.nodes) item.timestamp = "1000"
			}
			const request = vi.fn().mockResolvedValue(data)
			const gateway = Object.create(IntentGateway.prototype) as IntentGateway
			gateway.withQueryClient({ request } as unknown as IndexerQueryClient, { pollInterval: 10 })
			const stream = gateway.orderStatusStream(commitment)
			const result = stream.next()
			await vi.advanceTimersByTimeAsync(10)
			expect(await result).toMatchObject({ done: false, value: { status: OrderStatus.REFUNDED } })
			expect(await stream.next()).toEqual({ done: true, value: undefined })
			expect(request).toHaveBeenCalledTimes(1)
		},
	)

	it("emits cancellation once while waiting for a refund across repeated polls", async () => {
		const request = vi
			.fn()
			.mockResolvedValueOnce(response([OrderStatus.PLACED]))
			.mockResolvedValueOnce(response([OrderStatus.PLACED, OrderStatus.CANCELLED]))
			.mockResolvedValueOnce(response([OrderStatus.PLACED, OrderStatus.CANCELLED]))
			.mockResolvedValueOnce(response([OrderStatus.PLACED, OrderStatus.CANCELLED, OrderStatus.REFUNDED]))
		const gateway = Object.create(IntentGateway.prototype) as IntentGateway
		gateway.withQueryClient({ request } as unknown as IndexerQueryClient, { pollInterval: 10 })
		const stream = gateway.orderStatusStream(commitment)
		const placed = stream.next()
		await vi.advanceTimersByTimeAsync(10)
		expect(await placed).toMatchObject({ value: { status: OrderStatus.PLACED } })
		const cancelled = stream.next()
		await vi.advanceTimersByTimeAsync(10)
		expect(await cancelled).toMatchObject({ value: { status: OrderStatus.CANCELLED } })
		const refunded = stream.next()
		await vi.advanceTimersByTimeAsync(20)
		expect(await refunded).toMatchObject({ value: { status: OrderStatus.REFUNDED } })
		expect(await stream.next()).toEqual({ done: true, value: undefined })
		expect(request).toHaveBeenCalledTimes(4)
	})

	it.each([OrderStatus.REFUNDED, OrderStatus.FILLED, OrderStatus.REDEEMED])(
		"continues after CANCELLED and stops at %s",
		async (terminal) => {
			const request = vi
				.fn()
				.mockResolvedValueOnce(response([OrderStatus.PLACED, OrderStatus.CANCELLED]))
				.mockResolvedValueOnce(response([OrderStatus.PLACED, OrderStatus.CANCELLED, terminal]))
			// Indexer-only methods do not need on-chain clients or contract discovery.
			const gateway = Object.create(IntentGateway.prototype) as IntentGateway
			gateway.withQueryClient({ request } as unknown as IndexerQueryClient, { pollInterval: 10 })
			const stream = gateway.orderStatusStream(commitment)
			const cancelled = stream.next()
			await vi.advanceTimersByTimeAsync(10)
			expect(await cancelled).toMatchObject({ done: false, value: { status: OrderStatus.CANCELLED } })
			const settled = stream.next()
			await vi.advanceTimersByTimeAsync(10)
			expect(await settled).toMatchObject({ done: false, value: { status: terminal } })
			expect(await stream.next()).toEqual({ done: true, value: undefined })
			expect(request).toHaveBeenCalledTimes(2)
		},
	)
})
