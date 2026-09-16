import { afterEach, describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { OrderbookRequestError } from "@/orderbook/client"
import { LimitOrderLifecycle } from "@/orderbook/lifecycle"
import type { SubmitOrderResult } from "@/orderbook/types"
import {
	CREATE_REQUEST as REQUEST,
	fakeClient,
	limitOrderService as makeService,
	ORDERBOOK_FIXTURES,
	postedOrder,
} from "../helpers/limit-orders"

const { ONE } = ORDERBOOK_FIXTURES

/** Past every grace period, for a reconciliation that should treat a row as settled. */
function later(): Date {
	return new Date(Date.now() + 10 * 60 * 1000)
}

/** An ISO stamp `secs` from now, which is how the orderbook reports an expiry. */
function inSeconds(secs: number): string {
	return new Date(Date.now() + secs * 1000).toISOString()
}

/** A client that also records the entries it was asked to cancel. */
function countingClient(...args: Parameters<typeof fakeClient>) {
	const client = fakeClient(...args)
	const cancelled: string[] = []
	const inner = client.cancelOrder
	client.cancelOrder = async (params) => {
		cancelled.push(params.commitment)
		return inner(params)
	}
	return Object.assign(client, { cancelled })
}

describe("heartbeat", () => {
	it("says nothing to an orderbook that has never taken an order from us", async () => {
		// A solver the orderbook has not met can only be answered `UNKNOWN_SOLVER`.
		const client = fakeClient([])
		const { service } = makeService(client)

		expect(await service.heartbeat()).toBeNull()
		expect(client.heartbeats).toHaveLength(0)
	})

	it("signs one once an order is posted", async () => {
		const client = fakeClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)

		expect(await service.heartbeat()).toMatchObject({ kind: "accepted", status: "ACTIVE" })
		expect(client.heartbeats).toHaveLength(1)
	})

	it("goes out on the back of a posting the orderbook has not surfaced", async () => {
		// A new solver stays suspended until it is heard from, and the posting that
		// made it known is what makes the heartbeat answerable.
		const client = fakeClient([{ kind: "accepted", order: postedOrder(), surfaced: false }])
		const { service } = makeService(client)
		await service.create(REQUEST)

		expect(client.heartbeats).toHaveLength(1)
	})

	it("re-signs on a later second when the server has already seen this one", async () => {
		const client = fakeClient([])
		client.heartbeatResults = [{ kind: "rejected", code: "SIGNATURE_REUSED", message: "same second" }]
		const { service } = makeService(client)
		await service.create(REQUEST)

		expect(await service.heartbeat()).toMatchObject({ kind: "accepted" })
		expect(client.heartbeats).toHaveLength(2)
		expect(client.heartbeats[1]).toBeGreaterThan(client.heartbeats[0])
	})

	it("does not retry a refusal a later timestamp cannot fix", async () => {
		const client = fakeClient([])
		client.heartbeatResults = [{ kind: "rejected", code: "SOLVER_MISMATCH", message: "wrong key" }]
		const { service } = makeService(client)
		await service.create(REQUEST)

		expect(await service.heartbeat()).toMatchObject({ kind: "rejected", code: "SOLVER_MISMATCH" })
		expect(client.heartbeats).toHaveLength(1)
	})

	it("asks half as often as the server requires, so one lost request is not a suspension", async () => {
		const { service } = makeService(fakeClient([]))
		expect(await service.heartbeatIntervalMs()).toBe(30_000)
	})
})

describe("renewal", () => {
	const withExpiry = async (expiresAt: string) => {
		const client = fakeClient([{ kind: "accepted", order: postedOrder({ expiresAt }), surfaced: true }])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		return { client, service, store, id: created.order.id }
	}

	it("posts a fresh op on a new nonce before the entry expires", async () => {
		// The op carries its own deadline and the orderbook remembers its hash, so
		// renewal cannot be the same op sent again.
		const { client, service, store, id } = await withExpiry(inSeconds(60))

		expect(await service.renewExpiring(120)).toBe(1)
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect((await store.get(id))?.orderNonce).toBe("1")
	})

	it("leaves a posting with time on it alone", async () => {
		const { client, service } = await withExpiry(inSeconds(3600))

		expect(await service.renewExpiring(120)).toBe(0)
		expect(client.submitted).toEqual(["0x00"])
	})
})

describe("the operator's own expiry", () => {
	/** An order expiring in an hour, and the clock wound past it. */
	const withExpiry = async (results: SubmitOrderResult[] = []) => {
		const client = countingClient(results)
		const { service, store } = makeService(client)
		const created = await service.create({ ...REQUEST, expiresAt: inSeconds(3600) })
		return { client, service, store, id: created.order.id, after: new Date(Date.now() + 2 * 3600 * 1000) }
	}

	it("withdraws an order that has outlived it", async () => {
		// The matcher already refuses an expired order, so a posting left up
		// advertises depth no swapper could ever draw on.
		const { client, service, store, id, after } = await withExpiry()

		expect(await service.expireStale(after)).toBe(1)
		const order = await store.get(id)
		expect(order?.status).toBe("expired")
		expect(order?.commitment).toBeNull()
		expect(client.cancelled).toEqual(["0xabc"])
	})

	it("leaves an order whose expiry has not come", async () => {
		const { client, service, store, id } = await withExpiry()

		expect(await service.expireStale()).toBe(0)
		expect((await store.get(id))?.status).toBe("open")
		expect(client.cancelled).toEqual([])
	})

	it("renews nothing once the order has expired", async () => {
		// The posting is due for renewal on its own clock; the sweep has to have
		// taken it down first, or renewal puts a fresh one up for a dead order.
		const { client, service, after } = await withExpiry([
			{ kind: "accepted", order: postedOrder({ expiresAt: inSeconds(60) }), surfaced: true },
		])

		await service.expireStale(after)
		expect(await service.renewExpiring(120)).toBe(0)
		expect(client.submitted).toEqual(["0x00"])
	})

	it("treats an expiry it cannot read as no expiry at all", async () => {
		// `create` refuses one it cannot parse, so this is a row from before that
		// check existed. The sweep must not guess at it either way.
		const client = countingClient([])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		await store.create({ ...created.order, id: "unreadable", expiresAt: "whenever" })

		expect(await service.expireStale()).toBe(0)
		expect((await store.get("unreadable"))?.status).toBe("open")
	})
})

describe("reconciliation", () => {
	it("cancels an orderbook entry no limit order here owns", async () => {
		// Orphaned by a crash, or by a local cancel whose request never landed.
		const client = countingClient([])
		client.entries = [postedOrder({ commitment: "0xdead" as HexString })]
		const { service } = makeService(client)

		expect(await service.reconcile()).toEqual({ cancelled: 1, reposted: 0, underFunded: 0 })
		expect(client.cancelled).toHaveLength(1)
	})

	it("posts a limit order again when its entry has gone", async () => {
		const client = countingClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)

		// From later on: a row touched moments ago has a posting in flight.
		expect(await service.reconcile(later())).toEqual({ cancelled: 0, reposted: 1, underFunded: 0 })
		// No cancel first: the entry the orderbook would be asked about is the one
		// it has just said it does not have.
		expect(client.cancelled).toHaveLength(0)
		expect(client.submitted).toEqual(["0x00", "0x01"])
	})

	it("leaves an order alone while its entry is where it should be", async () => {
		const client = countingClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)
		client.entries = [postedOrder()]

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.submitted).toEqual(["0x00"])
	})

	it("surfaces a posting the orderbook cut down, without reposting it", async () => {
		const client = countingClient([])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		client.entries = [postedOrder({ resized: true, advertisedSize: "1" })]

		expect(await service.reconcile()).toMatchObject({ reposted: 0, underFunded: 1 })
		const order = await store.get(created.order.id)
		expect(order?.status).toBe("open")
		expect(order?.lastError).toMatch(/^UNDER_FUNDED: the orderbook is advertising 1 /)
		expect(client.submitted).toEqual(["0x00"])
	})

	it("surfaces a posting a balance cycle found the solver cannot cover", async () => {
		const client = countingClient([])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		client.entries = [postedOrder({ backed: false, validatedAt: "2026-09-16T10:00:00.000Z" })]

		expect(await service.reconcile()).toMatchObject({ underFunded: 1 })
		expect((await store.get(created.order.id))?.lastError).toMatch(/balance does not cover this posting/)
	})

	it("says nothing about a posting no cycle has reached yet", async () => {
		// `backed` is false before any balance has been read, and the posting
		// surfaces at its full size meanwhile, so a fresh one would otherwise be
		// reported as under-funded within seconds of going up.
		const client = countingClient([])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		client.entries = [postedOrder({ backed: false, validatedAt: null })]

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect((await store.get(created.order.id))?.lastError).toBeNull()
	})

	it("waits out a repost that is still in flight rather than posting a second entry", async () => {
		const client = countingClient([])
		const { service, store } = makeService(client)
		const created = await service.create(REQUEST)
		await store.setStatus(created.order.id, "resizing")

		expect(await service.reconcile()).toMatchObject({ reposted: 0 })
		expect(client.submitted).toEqual(["0x00"])
	})

	it("waits out a posting that has not answered yet, whatever opened the window", async () => {
		// A create posts after its insert and a renewal posts after its cancel, and
		// both leave a live row with no entry to find. Reading that as one to put
		// back is how two entries end up behind one liability.
		const client = countingClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.submitted).toEqual(["0x00"])
	})
})

/**
 * The three clocks against one order, in the sequence a live filler puts them
 * in. Each one leaves the order in the state the next one reads, and the pair
 * that can fight over a single liability is a repost and a reconciliation.
 */
describe("a posting through a fill, a renewal and a reconciliation", () => {
	it("ends with one entry on the book and nothing to repair", async () => {
		const client = countingClient([
			{ kind: "accepted", order: postedOrder({ commitment: "0xa1", expiresAt: inSeconds(60) }), surfaced: true },
			{ kind: "accepted", order: postedOrder({ commitment: "0xa2", expiresAt: inSeconds(60) }), surfaced: true },
			{ kind: "accepted", order: postedOrder({ commitment: "0xa3", expiresAt: inSeconds(3600) }), surfaced: true },
		])
		const { service, store } = makeService(client)
		const { order } = await service.create(REQUEST)

		// A fill takes 500,000 of the 1,500,000 cNGN offered, and the rest goes back
		// up at its new size.
		const resized = (await service.settleFill(order.id, 500_000n * ONE))!
		expect(resized.commitment).toBe("0xa2")
		expect(resized.remaining).toBe((1_000_000n * ONE).toString())

		// That new entry expires inside the margin, so renewal replaces it.
		expect(await service.renewExpiring(120)).toBe(1)
		const renewed = (await store.get(order.id))!
		expect(renewed.commitment).toBe("0xa3")
		expect(renewed.remaining).toBe(resized.remaining)

		// Three postings, each a fresh op on a fresh nonce, and each one cancelled
		// before its replacement went up.
		expect(client.submitted).toEqual(["0x00", "0x01", "0x02"])
		expect(renewed.orderNonce).toBe("2")
		expect(client.cancelled).toEqual(["0xa1", "0xa2"])

		// The orderbook now holds exactly what the row says it does.
		client.entries = [postedOrder({ commitment: "0xa3" })]
		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.submitted).toHaveLength(3)
		expect((await store.get(order.id))?.status).toBe("open")
	})
})

describe("LimitOrderLifecycle", () => {
	afterEach(() => vi.useRealTimers())

	it("runs the clocks until it is stopped", async () => {
		const client = fakeClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)
		// Warms the limits cache, which the heartbeat reads to sign.
		await service.heartbeatIntervalMs()
		client.heartbeats.length = 0

		vi.useFakeTimers()
		const lifecycle = new LimitOrderLifecycle(service, { renewMarginSecs: 120, reconcileIntervalSecs: 300 })
		await lifecycle.start()

		await vi.advanceTimersByTimeAsync(60_000)
		expect(client.heartbeats).toHaveLength(2)

		lifecycle.stop()
		await vi.advanceTimersByTimeAsync(300_000)
		expect(client.heartbeats).toHaveLength(2)
	})

	it("reconciles on the way up rather than waiting out the first interval", async () => {
		const client = fakeClient([])
		const { service } = makeService(client)
		await service.create(REQUEST)
		let walks = 0
		const inner = client.myOrders.bind(client)
		client.myOrders = async () => {
			walks += 1
			return inner()
		}

		const lifecycle = new LimitOrderLifecycle(service, { renewMarginSecs: 120, reconcileIntervalSecs: 300 })
		await lifecycle.start()
		lifecycle.stop()

		await vi.waitFor(() => expect(walks).toBe(1))
	})

	it("starts against an orderbook that is not answering", async () => {
		// The filler prices from the local limit orders either way, and the clocks
		// are what bring the postings back once it answers again.
		const client = fakeClient([])
		client.limits = async () => {
			throw new OrderbookRequestError("connect ECONNREFUSED")
		}
		const { service } = makeService(client)

		const lifecycle = new LimitOrderLifecycle(service, { renewMarginSecs: 120, reconcileIntervalSecs: 300 })
		await expect(lifecycle.start()).resolves.toBeUndefined()
		lifecycle.stop()
	})
})
