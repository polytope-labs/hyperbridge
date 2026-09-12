;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
;(global as any).chainId = "8453"

const records = new Map<string, any>()
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => {
		records.set(`${entity}:${id}`, { ...props })
	}),
	getByField: jest.fn(async (entity: string, field: string, value: unknown, options: any = {}) => {
		const entries = [...records.entries()]
			.filter(([key, props]) => key.startsWith(`${entity}:`) && props[field] === value)
			.map(([, props]) => props)
		const offset = options.offset ?? 0
		return entries.slice(offset, offset + (options.limit ?? entries.length))
	}),
	getByFields: jest.fn(async () => []),
	remove: jest.fn(async (entity: string, id: string) => records.delete(`${entity}:${id}`)),
}

import { OrderStatus } from "@/configs/src/types"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"

const COMMITMENT = "0x0000000000000000000000000000000000000000000000000000000000000001"
const TX_HASH = "0x0000000000000000000000000000000000000000000000000000000000000002"
const CANCELLER = "0x0000000000000000000000000000000000000003"

const placeOrder = (status: OrderStatus) => {
	records.set(`IOrderV3:${COMMITMENT}`, {
		id: COMMITMENT,
		user: "0x0000000000000000000000000000000000000000000000000000000000000004",
		sourceChain: "EVM-8453",
		destChain: "EVM-56",
		commitment: COMMITMENT,
		deadline: 100n,
		nonce: 1n,
		fees: 0n,
		inputUSD: 0n,
		status,
		predispatchCalldata: "0x",
		postDispatchCalldata: "0x",
		createdAt: new Date(0),
		blockNumber: 1n,
		blockTimestamp: 1n,
		transactionHash: TX_HASH,
	})
}

const event = { transactionHash: TX_HASH, blockNumber: 123, timestamp: 456n, logIndex: 7 }

describe("IntentGatewayV3Service.recordOrderCancellation", () => {
	beforeEach(() => records.clear())

	it("records the cancellation and moves a placed order to CANCELLED", async () => {
		placeOrder(OrderStatus.PLACED)

		await IntentGatewayV3Service.recordOrderCancellation(COMMITMENT, CANCELLER, event)

		expect(records.get(`IOrderV3:${COMMITMENT}`).status).toBe(OrderStatus.CANCELLED)
		expect(records.get(`IOrderV3Cancellation:${TX_HASH}.7`)).toMatchObject({
			orderId: COMMITMENT,
			chain: "8453",
			canceller: CANCELLER,
			timestamp: 456n,
			blockNumber: "123",
		})
		expect(records.get(`IOrderV3StatusMetadata:${COMMITMENT}.${OrderStatus.CANCELLED}`)).toMatchObject({
			orderId: COMMITMENT,
			status: OrderStatus.CANCELLED,
		})
	})

	it("records a late cancellation without regressing a refunded order", async () => {
		placeOrder(OrderStatus.REFUNDED)

		await IntentGatewayV3Service.recordOrderCancellation(COMMITMENT, CANCELLER, event)

		expect(records.get(`IOrderV3:${COMMITMENT}`).status).toBe(OrderStatus.REFUNDED)
		expect(records.get(`IOrderV3Cancellation:${TX_HASH}.7`)).toBeDefined()
		expect(records.get(`IOrderV3StatusMetadata:${COMMITMENT}.${OrderStatus.CANCELLED}`)).toBeUndefined()
	})

	it("uses the established pending-status path when the order has not arrived", async () => {
		await IntentGatewayV3Service.recordOrderCancellation(COMMITMENT, CANCELLER, event)

		expect(records.get(`IOrderV3Cancellation:${TX_HASH}.7`)).toBeDefined()
		expect(records.get(`PendingStatusMetadata:${COMMITMENT}.IOrderV3.${OrderStatus.CANCELLED}`)).toMatchObject({
			commitment: COMMITMENT,
			status: OrderStatus.CANCELLED,
			chain: "8453",
		})
	})
})
