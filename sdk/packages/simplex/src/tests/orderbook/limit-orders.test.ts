import { describe, expect, it } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { MemoryDataStore } from "@/data/memory"
import { ORDERBOOK_SCALE } from "@/orderbook/amounts"
import { OrderbookRequestError } from "@/orderbook/client"
import { LimitOrderService, LimitOrderValidationError, type CreateLimitOrderRequest } from "@/orderbook/limit-orders"
import type { CancelOrderResult, OrderbookLimits, PostedOrder, SubmitOrderResult } from "@/orderbook/types"

const CHAIN = "EVM-8453"
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const CNGN = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString
const ONE = ORDERBOOK_SCALE

const LIMITS: OrderbookLimits = {
	serverInfo: {
		minOrderTtlSecs: 900,
		heartbeatIntervalSecs: 60,
		signatureSkewSecs: 30,
		maxBatchSize: 20,
		minOrderSizes: [{ symbol: "CNGN", size: (1000n * ONE).toString() }],
		eip712DomainName: "HyperFX Orderbook",
		eip712DomainVersion: "1",
	},
	books: [{ id: "USDC/CNGN", base: "USDC", quote: "CNGN" }],
}

function postedOrder(overrides: Partial<PostedOrder> = {}): PostedOrder {
	return {
		commitment: "0xabc" as HexString,
		side: "BID",
		status: "ACTIVE",
		price: (1490n * ONE).toString(),
		quotedAmount: (1_500_000n * ONE).toString(),
		advertisedSize: (1_500_000n * ONE).toString(),
		expiresAt: "2026-09-15T12:00:00.000Z",
		acceptedSources: ["EVM-1"],
		...overrides,
	}
}

/** An orderbook that answers from a queue, and records what it was sent. */
function fakeClient(results: SubmitOrderResult[], cancels: CancelOrderResult[] = []) {
	const submitted: HexString[] = []
	return {
		submitted,
		limits: async () => LIMITS,
		submitOrder: async (userOp: HexString) => {
			submitted.push(userOp)
			return results.shift() ?? { kind: "accepted" as const, order: postedOrder(), surfaced: true }
		},
		cancelOrder: async () =>
			cancels.shift() ?? ({ kind: "cancelled", commitment: "0xabc" as HexString } as CancelOrderResult),
	}
}

function makeService(client: ReturnType<typeof fakeClient>, store = new MemoryDataStore().limitOrders) {
	const contractService = {
		getTokenDecimals: async (token: string) => (token === USDC ? 6 : 18),
		// The op is opaque to the service; the nonce is echoed so a repost is visible.
		prepareLimitOrderUserOp: async ({ orderNonce }: { orderNonce: bigint }) => ({
			commitment: "0xabc" as HexString,
			userOp: `0x0${orderNonce}` as HexString,
		}),
	}
	const configService = {
		getConfiguredChainIds: () => [8453],
		getEntryPointAddress: () => "0x4444444444444444444444444444444444444444" as HexString,
	}
	const assetRegistry = {
		getAddress: (symbol: string, chain: string) =>
			chain === CHAIN ? ({ USDC, CNGN } as Record<string, HexString>)[symbol] ?? null : null,
	}
	const signer = { address: SOLVER, signTypedData: async () => "0xsig" as HexString }

	// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for the collaborators this path touches
	const service = new LimitOrderService(
		store,
		client as any,
		contractService as any,
		configService as any,
		assetRegistry as any,
		signer as any,
		900,
		undefined,
	)
	return { service, store }
}

const REQUEST: CreateLimitOrderRequest = {
	book: "USDC/CNGN",
	side: "BID",
	fillChain: CHAIN,
	price: (1500n * ONE).toString(),
	size: (1_500_000n * ONE).toString(),
	acceptedSources: ["EVM-1"],
}

describe("LimitOrderService.create", () => {
	it("stores the order and records the orderbook's posting", async () => {
		const { service, store } = makeService(fakeClient([]))
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("accepted")
		expect(order.status).toBe("open")
		expect(order.commitment).toBe("0xabc")
		// The orderbook shades a posting by the protocol fee, so both prices are kept.
		expect(order.price).toBe((1500n * ONE).toString())
		expect(order.bookPrice).toBe((1490n * ONE).toString())
		expect(order.remaining).toBe(order.size)
		expect(await store.get(order.id)).toEqual(order)
	})

	it("keeps a rejected order with the reason on it rather than dropping the request", async () => {
		const client = fakeClient([{ kind: "rejected", code: "UNSUPPORTED_PAIR", message: "no such market" }])
		const { service, store } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("rejected")
		expect(order.status).toBe("rejected")
		expect(order.lastError).toBe("UNSUPPORTED_PAIR: no such market")
		expect(await store.get(order.id)).not.toBeNull()
	})

	it("bumps the nonce and reposts once when the orderbook has seen the op before", async () => {
		const client = fakeClient([{ kind: "rejected", code: "REPLAYED", message: "seen" }])
		const { service } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("accepted")
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(order.orderNonce).toBe("1")
		expect(order.status).toBe("open")
	})

	it("does not retry a rejection a new nonce cannot fix", async () => {
		const client = fakeClient([{ kind: "rejected", code: "BAD_SIGNATURE", message: "bad" }])
		const { service } = makeService(client)
		await service.create(REQUEST)
		expect(client.submitted).toHaveLength(1)
	})

	it("reports an unreachable orderbook as retryable and leaves the order rejected", async () => {
		const client = fakeClient([])
		client.submitOrder = async () => {
			throw new OrderbookRequestError("connect ECONNREFUSED")
		}
		const { service } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result).toMatchObject({ kind: "failed", retryable: true })
		expect(order.status).toBe("rejected")
	})
})

describe("LimitOrderService.create validation", () => {
	const rejects = async (patch: Partial<CreateLimitOrderRequest>, match: RegExp) => {
		const { service } = makeService(fakeClient([]))
		await expect(service.create({ ...REQUEST, ...patch })).rejects.toThrow(match)
	}

	it("refuses a missing or empty accepted-sources list, which the orderbook would reject", async () => {
		await rejects({ acceptedSources: [] }, /at least one source chain/)
		await rejects({ acceptedSources: undefined as unknown as string[] }, /at least one source chain/)
	})

	it("refuses a repeated source chain", async () => {
		await rejects({ acceptedSources: ["EVM-1", "EVM-1"] }, /must not repeat/)
	})

	it("refuses a size under the output token's dust floor", async () => {
		await rejects({ size: (999n * ONE).toString() }, /dust floor for CNGN/)
	})

	it("refuses a ttl under the orderbook's minimum", async () => {
		await rejects({ ttlSecs: 60 }, /at least the orderbook's minimum of 900/)
	})

	it("refuses an unknown book, naming the ones on offer", async () => {
		await rejects({ book: "USDC/EURC" }, /Unknown book 'USDC\/EURC'.*USDC\/CNGN/)
	})

	it("refuses a chain this filler does not run", async () => {
		await rejects({ fillChain: "EVM-1" }, /not a chain this filler is configured for/)
	})

	it("refuses a price or size that is not a positive 1e18 integer", async () => {
		await rejects({ price: "0" }, /price must be a positive integer/)
		await rejects({ size: "1.5" }, /size must be a positive integer/)
	})
})

describe("LimitOrderService.cancel", () => {
	it("cancels locally first, then clears the orderbook entry", async () => {
		const { service, store } = makeService(fakeClient([]))
		const created = await service.create(REQUEST)

		const { order, result } = await service.cancel(created.order.id)
		expect(result.kind).toBe("cancelled")
		expect(order.status).toBe("cancelled")
		expect(order.commitment).toBeNull()
		expect((await store.get(order.id))?.status).toBe("cancelled")
	})

	it("treats an entry the orderbook no longer knows as already cancelled", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" }])
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const { order } = await service.cancel(created.order.id)
		expect(order.status).toBe("cancelled")
		expect(order.commitment).toBeNull()
		expect(order.lastError).toBeNull()
	})

	it("re-signs once on a timestamp the server would not take", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "SIGNATURE_REUSED", message: "same second" }])
		let cancels = 0
		const inner = client.cancelOrder
		client.cancelOrder = async () => {
			cancels += 1
			return inner()
		}
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const { result } = await service.cancel(created.order.id)
		expect(cancels).toBe(2)
		expect(result.kind).toBe("cancelled")
	})

	it("stays cancelled locally even when the orderbook refuses, with the refusal on the row", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "SOLVER_MISMATCH", message: "wrong key" }])
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const { order } = await service.cancel(created.order.id)
		expect(order.status).toBe("cancelled")
		expect(order.lastError).toBe("SOLVER_MISMATCH: wrong key")
	})

	it("refuses an id it does not know", async () => {
		const { service } = makeService(fakeClient([]))
		await expect(service.cancel("nope")).rejects.toThrow(LimitOrderValidationError)
	})
})
