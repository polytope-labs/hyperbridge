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
		chains: [CHAIN, "EVM-1"],
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
		/** Entries the orderbook already holds, by commitment. */
		entries: [] as PostedOrder[],
		limits: async () => LIMITS,
		orderAt: async function (_solver: HexString, commitment: HexString) {
			return this.entries.find((entry) => entry.commitment === commitment) ?? null
		},
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

/** Take in 1,000 USDC, pay out 1,500,000 cNGN: a USDC to cNGN order at 1,500. */
const REQUEST: CreateLimitOrderRequest = {
	fillChain: CHAIN,
	tokenIn: "USDC",
	amountIn: "1000",
	tokenOut: "CNGN",
	amountOut: "1500000",
	acceptedSources: ["EVM-1"],
}

describe("LimitOrderService.create", () => {
	it("stores the order and records the orderbook's posting", async () => {
		const { service, store } = makeService(fakeClient([]))
		const { order, result } = await service.create(REQUEST)

		expect(result.kind).toBe("accepted")
		expect(order.status).toBe("open")
		expect(order.commitment).toBe("0xabc")
		// The rate follows from the two amounts: 1,500,000 cNGN for 1,000 USDC.
		// The orderbook shades a posting by the protocol fee, so both are kept.
		expect(order.side).toBe("BID")
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

	it("leaves an order the orderbook could not decide open, to be posted again", async () => {
		// A refusal retires the order; a failure must not. One timeout on the way in
		// would otherwise kill an order the operator still wants, with nothing left
		// looking at the row.
		const client = fakeClient([])
		client.submitOrder = async () => {
			throw new OrderbookRequestError("connect ECONNREFUSED")
		}
		const { service, store } = makeService(client)
		const { order, result } = await service.create(REQUEST)

		expect(result).toMatchObject({ kind: "failed", retryable: true })
		expect(order.status).toBe("open")
		expect(order.commitment).toBeNull()
		expect(order.lastError).toMatch(/REQUEST_FAILED/)
		expect((await store.get(order.id))?.status).toBe("open")
	})

	it("retires an order the orderbook actually refused", async () => {
		const client = fakeClient([{ kind: "rejected", code: "BAD_SIGNATURE", message: "bad" }])
		const { service } = makeService(client)
		const { order } = await service.create(REQUEST)
		expect(order.status).toBe("rejected")
	})
})

describe("an op the orderbook has already taken", () => {
	it("treats a live entry at the same commitment as the posting it was", async () => {
		// `ORDER_EXISTS` is a live entry sitting at that commitment, unlike
		// `REPLAYED`. Posting again on a new nonce would leave two entries behind one
		// liability and record only the second.
		const client = fakeClient([{ kind: "rejected", code: "ORDER_EXISTS", message: "already here" }])
		client.entries = [postedOrder({ commitment: "0xabc" })]
		const { service } = makeService(client)

		const { order, result } = await service.create(REQUEST)
		expect(result.kind).toBe("unchanged")
		expect(order.status).toBe("open")
		expect(order.commitment).toBe("0xabc")
		// One submission, not two.
		expect(client.submitted).toEqual(["0x00"])
	})

	it("bumps the nonce when the orderbook holds no such entry", async () => {
		const client = fakeClient([{ kind: "rejected", code: "REPLAYED", message: "seen" }])
		const { service } = makeService(client)

		const { order } = await service.create(REQUEST)
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(order.orderNonce).toBe("1")
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

	it("refuses an amountOut under the paid token's dust floor", async () => {
		await rejects({ amountOut: "999" }, /dust floor for CNGN/)
	})

	it("refuses a ttl under the orderbook's minimum", async () => {
		await rejects({ ttlSecs: 60 }, /at least the orderbook's minimum of 900/)
	})

	it("refuses a pair no book trades, naming the ones on offer", async () => {
		await rejects({ tokenOut: "EURC" }, /No book trades USDC against EURC.*USDC\/CNGN/)
	})

	it("refuses an order that takes in and pays out the same symbol", async () => {
		await rejects({ tokenOut: "USDC" }, /must be different symbols/)
	})

	it("refuses a chain this filler does not run", async () => {
		await rejects({ fillChain: "EVM-1" }, /not a chain this filler is configured for/)
	})

	it("refuses a source chain the orderbook does not serve", async () => {
		// Half of what UNSUPPORTED_SOURCE_CHAIN tests, and the half an operator gets
		// wrong by typo, so it is worth catching before a row is stored.
		await rejects({ acceptedSources: ["EVM-1", "EVM-42161"] }, /does not serve EVM-42161/)
	})

	it("refuses an amount that is not a positive decimal in whole tokens", async () => {
		await rejects({ amountIn: "0" }, /amountIn must be greater than zero/)
		await rejects({ amountOut: "1.5e3" }, /amountOut must be an amount in whole tokens/)
		await rejects({ amountOut: "0.0000000000000000001" }, /more than 18 decimal places/)
	})
})

describe("LimitOrderService.settleFill", () => {
	/** Take in 1,000 USDC, pay out 1,500,000 cNGN, then see 500,000 cNGN go out. */
	const fill = async (delivered: bigint, cancels: CancelOrderResult[] = []) => {
		const client = fakeClient([], cancels)
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		const settled = await service.settleFill(created.order.id, delivered)
		return { client, store, settled: settled!, id: created.order.id }
	}

	it("reports a resize and a close, which nobody asked for", async () => {
		// Every other change to a limit order is something the operator initiated and
		// gets told about by the controller that took the request. These two happen
		// inside a fill, so without this the size just goes stale on their screen.
		const client = fakeClient([])
		const { service } = makeService(client)
		const events: string[] = []
		service.listen((event) => events.push(`${event.kind}:${event.order.remaining}`))
		const created = await service.create(REQUEST)

		await service.settleFill(created.order.id, 500_000n * ONE)
		expect(events).toEqual([`resized:${(1_000_000n * ONE).toString()}`])

		await service.settleFill(created.order.id, 999_999n * ONE)
		expect(events[1]).toMatch(/^filled:/)
	})

	it("works the order down by what went out and reposts the rest", async () => {
		const { client, settled } = await fill(500_000n * ONE)

		expect(settled.remaining).toBe((1_000_000n * ONE).toString())
		expect(settled.status).toBe("open")
		// A fresh op on a fresh nonce: the orderbook remembers every hash it took.
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(settled.orderNonce).toBe("1")
	})

	it("closes the order when what is left falls under the dust floor", async () => {
		// 1,499,500 of 1,500,000 delivered leaves 500, under the 1,000 floor.
		const { client, settled } = await fill(1_499_500n * ONE)

		expect(settled.status).toBe("filled")
		expect(settled.commitment).toBeNull()
		// Cancelled, not reposted.
		expect(client.submitted).toEqual(["0x00"])
	})

	it("cancels the old entry before posting the new one", async () => {
		// Two live entries for one liability would advertise the same output twice.
		const order: string[] = []
		const client = fakeClient([])
		const submit = client.submitOrder
		client.submitOrder = async (userOp) => {
			order.push("submit")
			return submit(userOp)
		}
		client.cancelOrder = async () => {
			order.push("cancel")
			return { kind: "cancelled", commitment: "0xabc" as HexString }
		}
		const { service } = makeService(client)
		const created = await service.create(REQUEST)
		await service.settleFill(created.order.id, 500_000n * ONE)

		expect(order).toEqual(["submit", "cancel", "submit"])
	})

	it("reposts even when the orderbook no longer knows the old entry", async () => {
		const { settled } = await fill(500_000n * ONE, [
			{ kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" },
		])
		expect(settled.status).toBe("open")
		expect(settled.commitment).toBe("0xabc")
	})

	it("does nothing for an order it does not know", async () => {
		const { service } = makeService(fakeClient([]))
		expect(await service.settleFill("missing", ONE)).toBeNull()
	})

	it("floors the draw-down at zero when a fill delivered more than was left", async () => {
		const { settled } = await fill(9_000_000n * ONE)
		expect(settled.remaining).toBe("0")
		expect(settled.status).toBe("filled")
	})
})

describe("reposting when the old entry will not come down", () => {
	it("leaves the posting alone rather than adding a second entry", async () => {
		// Cancel then post is the right order, but only if the cancel worked. A
		// refusal leaves the old entry live, and posting over it is how two entries
		// end up behind one liability.
		const client = fakeClient([], [{ kind: "rejected", code: "SOLVER_MISMATCH", message: "wrong key" }])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)

		const reposted = await service.repost(created.order)
		expect(reposted?.commitment).toBe(created.order.commitment)
		expect(reposted?.lastError).toMatch(/SOLVER_MISMATCH/)
		expect(client.submitted).toEqual(["0x00"])
		expect((await store.get(created.order.id))?.status).toBe("open")
	})

	it("goes ahead when the orderbook says the entry is already gone", async () => {
		const client = fakeClient([], [{ kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" }])
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		await service.repost(created.order)
		expect(client.submitted).toEqual(["0x00", "0x01"])
	})

	it("holds off when the cancel never got an answer", async () => {
		const client = fakeClient([])
		client.cancelOrder = async () => ({ kind: "failed", message: "connect ECONNREFUSED" })
		const { service } = makeService(client)
		const created = await service.create(REQUEST)

		const reposted = await service.repost(created.order)
		expect(reposted?.lastError).toMatch(/ECONNREFUSED/)
		expect(client.submitted).toEqual(["0x00"])
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
