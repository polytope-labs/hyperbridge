import { afterEach, describe, expect, it } from "vitest"
import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { ENTRY_POINT_V08, ENTRY_POINT_V09, type HexString } from "@hyperbridge/sdk"
import { SqliteDataStore } from "@/data/sqlite"
import type { LimitOrderStore } from "@/data/types"
import { LoggerContext } from "@/services/Logger"
import {
	CREATE_REQUEST as REQUEST,
	fakeClient,
	limitOrderService,
	ORDERBOOK_FIXTURES,
	postedOrder,
} from "../helpers/limit-orders"

const { ONE } = ORDERBOOK_FIXTURES

/**
 * A posted op is signed over one EntryPoint. When a fill chain moves to another,
 * reconciliation withdraws every entry signed for the old one and posts it again
 * for the new.
 */

const dirs: string[] = []

afterEach(() => {
	for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true })
})

/** Past every grace period, for a reconciliation that should treat a row as settled. */
function later(): Date {
	return new Date(Date.now() + 10 * 60 * 1000)
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

/**
 * A service whose fill chain can be moved to another EntryPoint, recording the
 * EntryPoint each op it posts was signed for and the output it pays.
 */
function cutover(client: ReturnType<typeof countingClient>, store?: LimitOrderStore) {
	const made = limitOrderService(client, store)
	const chain = { entryPoint: ENTRY_POINT_V08 }
	const signedFor: string[] = []
	const paid: bigint[] = []
	const internals = made.service as any
	internals.configService.getEntryPointAddress = () => chain.entryPoint
	const prepare = internals.contractService.prepareLimitOrderUserOp
	internals.contractService.prepareLimitOrderUserOp = async (params: {
		entryPointAddress: HexString
		orderNonce: bigint
		outputAmount: bigint
	}) => {
		signedFor.push(params.entryPointAddress)
		paid.push(params.outputAmount)
		return prepare(params)
	}
	return { ...made, chain, signedFor, paid }
}

const accepted = (commitment: HexString) => ({
	kind: "accepted" as const,
	order: postedOrder({ commitment }),
	surfaced: true,
})

describe("a posting signed for an EntryPoint the chain has moved off", () => {
	it("is withdrawn and posted again for the new EntryPoint, on a fresh nonce, once", async () => {
		const client = countingClient([accepted("0xa1"), accepted("0xa2")])
		const { service, store, chain, signedFor } = cutover(client)
		const { order } = await service.create(REQUEST)
		expect(order.entryPoint).toBe(ENTRY_POINT_V08)

		client.entries = [postedOrder({ commitment: "0xa1" })]
		chain.entryPoint = ENTRY_POINT_V09

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 1, underFunded: 0 })
		expect(client.cancelled).toEqual(["0xa1"])
		expect(client.submitted).toEqual(["0x00", "0x01"])
		expect(signedFor).toEqual([ENTRY_POINT_V08, ENTRY_POINT_V09])
		expect(await store.get(order.id)).toMatchObject({
			status: "open",
			commitment: "0xa2",
			entryPoint: ENTRY_POINT_V09,
			orderNonce: "1",
			lastError: null,
		})

		client.entries = [postedOrder({ commitment: "0xa2" })]
		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.cancelled).toEqual(["0xa1"])
		expect(client.submitted).toHaveLength(2)
	})

	it("is read as signed for v0.8 when it was stored without an EntryPoint", async () => {
		const client = countingClient([accepted("0xa1"), accepted("0xa2")])
		const { service, store, chain, signedFor } = cutover(client)
		const { order } = await service.create(REQUEST)
		// How a posting stored before EntryPoints were recorded reads.
		await store.setPosting(order.id, {
			commitment: "0xa1",
			bookExpiresAt: null,
			bookPrice: null,
			orderNonce: order.orderNonce,
			entryPoint: null,
			status: "open",
			lastError: null,
		})
		client.entries = [postedOrder({ commitment: "0xa1" })]

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.cancelled).toEqual([])

		chain.entryPoint = ENTRY_POINT_V09
		expect(await service.reconcile()).toMatchObject({ reposted: 1 })
		expect(client.cancelled).toEqual(["0xa1"])
		expect(signedFor).toEqual([ENTRY_POINT_V08, ENTRY_POINT_V09])
		expect((await store.get(order.id))?.entryPoint).toBe(ENTRY_POINT_V09)
	})

	it("is left alone when it was signed for the chain's current EntryPoint, however that is cased", async () => {
		const client = countingClient([])
		const { service, chain, signedFor } = cutover(client)
		chain.entryPoint = ENTRY_POINT_V09
		await service.create(REQUEST)
		client.entries = [postedOrder()]
		chain.entryPoint = ENTRY_POINT_V09.toLowerCase() as HexString

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })
		expect(client.cancelled).toEqual([])
		expect(signedFor).toEqual([ENTRY_POINT_V09])
	})

	it("stays up when the old entry will not come down, and is tried again on the next pass", async () => {
		const client = countingClient(
			[accepted("0xa1"), accepted("0xa2")],
			[{ kind: "rejected", code: "SOLVER_MISMATCH", message: "wrong key" }],
		)
		const { service, store, chain } = cutover(client)
		const { order } = await service.create(REQUEST)
		client.entries = [postedOrder({ commitment: "0xa1" })]
		chain.entryPoint = ENTRY_POINT_V09

		expect(await service.reconcile()).toMatchObject({ reposted: 0 })
		expect(client.submitted).toEqual(["0x00"])
		const kept = await store.get(order.id)
		expect(kept).toMatchObject({ status: "open", commitment: "0xa1", entryPoint: ENTRY_POINT_V08 })
		expect(kept?.lastError).toMatch(/SOLVER_MISMATCH/)

		expect(await service.reconcile()).toMatchObject({ reposted: 1 })
		expect(await store.get(order.id)).toMatchObject({
			commitment: "0xa2",
			entryPoint: ENTRY_POINT_V09,
			lastError: null,
		})
	})

	it("is left to a repost that claimed the row after reconciliation listed it", async () => {
		const client = countingClient([accepted("0xa1")])
		const { service, store, chain } = cutover(client)
		const { order } = await service.create(REQUEST)
		client.entries = [postedOrder({ commitment: "0xa1" })]
		chain.entryPoint = ENTRY_POINT_V09

		// A fill's repost takes the row while the orderbook is still answering.
		const walk = client.myOrders.bind(client)
		client.myOrders = async () => {
			await Promise.resolve()
			await store.setStatus(order.id, "resizing")
			return walk()
		}

		expect(await service.reconcile()).toMatchObject({ reposted: 0 })
		expect(client.cancelled).toEqual([])
		expect(client.submitted).toEqual(["0x00"])
	})

	it("leaves one entry, at the fill's size, when a fill lands while the old entry is coming down", async () => {
		const client = countingClient([accepted("0xa1"), accepted("0xa2"), accepted("0xa3")])
		const { service, store, chain, signedFor, paid } = cutover(client)
		const { order } = await service.create(REQUEST)
		client.entries = [postedOrder({ commitment: "0xa1" })]
		chain.entryPoint = ENTRY_POINT_V09

		// The fill is drawn down and runs as far as it can before the withdrawal answers.
		let fill: Promise<unknown> | undefined
		const cancel = client.cancelOrder
		client.cancelOrder = async (params) => {
			if (!fill) {
				fill = service.settleFill(order.id, 500_000n * ONE)
				await new Promise((resolve) => setTimeout(resolve, 0))
			}
			return cancel(params)
		}

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 1, underFunded: 0 })
		await fill

		// Everything posted after the create was withdrawn but the last.
		expect(client.submitted).toEqual(["0x00", "0x01", "0x02"])
		expect(client.cancelled).toEqual(["0xa1", "0xa2"])
		expect(signedFor).toEqual([ENTRY_POINT_V08, ENTRY_POINT_V09, ENTRY_POINT_V09])
		expect(paid).toEqual([1_500_000n * ONE, 1_500_000n * ONE, 1_000_000n * ONE])
		expect(await store.get(order.id)).toMatchObject({
			status: "open",
			commitment: "0xa3",
			entryPoint: ENTRY_POINT_V09,
			orderNonce: "2",
			remaining: (1_000_000n * ONE).toString(),
		})
	})

	it("stays closed when a fill takes it under the dust floor while reconciliation reaches it", async () => {
		const client = countingClient([accepted("0xa1"), accepted("0xa2")])
		const { service, store, chain } = cutover(client)
		const events: string[] = []
		let closed!: () => void
		const filled = new Promise<void>((resolve) => (closed = resolve))
		service.listen((event) => {
			events.push(event.kind)
			if (event.kind === "filled") closed()
		})
		const { order } = await service.create(REQUEST)
		client.entries = [postedOrder({ commitment: "0xa1" })]
		chain.entryPoint = ENTRY_POINT_V09

		// The fill's withdrawal is still out when a pass that listed the old entry
		// reaches the order, and a re-sign's withdrawal would only answer once the
		// fill had closed the row.
		let pass: Promise<unknown> | undefined
		const cancel = client.cancelOrder
		client.cancelOrder = async (params) => {
			if (pass) {
				await filled
				return { kind: "rejected", code: "UNKNOWN_ORDER", message: "gone" }
			}
			pass = service.reconcile()
			await new Promise((resolve) => setTimeout(resolve, 0))
			return cancel(params)
		}

		await service.settleFill(order.id, 1_499_500n * ONE)
		await pass

		expect(events).toEqual(["filled"])
		expect(client.submitted).toEqual(["0x00"])
		expect(client.cancelled).toEqual(["0xa1"])
		expect(await store.get(order.id)).toMatchObject({ status: "filled", commitment: null, entryPoint: null })
	})

	it("is finished after a restart that came between the withdrawal and the new posting", async () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-entry-point-"))
		dirs.push(dir)
		const loggers = new LoggerContext({ level: "warn" })

		const first = new SqliteDataStore(dir, loggers)
		const before = countingClient([accepted("0xa1")])
		const old = cutover(before, first.limitOrders)
		const { order } = await old.service.create(REQUEST)
		before.entries = [postedOrder({ commitment: "0xa1" })]
		old.chain.entryPoint = ENTRY_POINT_V09
		before.submitOrder = async () => {
			throw new Error("the process died")
		}

		expect(await old.service.reconcile()).toMatchObject({ reposted: 0 })
		expect(before.cancelled).toEqual(["0xa1"])
		expect(await first.limitOrders.get(order.id)).toMatchObject({
			status: "resizing",
			commitment: "0xa1",
			entryPoint: ENTRY_POINT_V08,
		})
		await first.close()

		const second = new SqliteDataStore(dir, loggers)
		const after = countingClient([accepted("0xa2")])
		const restarted = cutover(after, second.limitOrders)
		restarted.chain.entryPoint = ENTRY_POINT_V09

		// The row was touched moments ago, so its posting may still be in flight.
		expect(await restarted.service.reconcile()).toMatchObject({ reposted: 0 })
		expect(await restarted.service.reconcile(later())).toEqual({ cancelled: 0, reposted: 1, underFunded: 0 })
		expect(after.cancelled).toEqual([])
		expect(restarted.signedFor).toEqual([ENTRY_POINT_V09])
		expect(await second.limitOrders.get(order.id)).toMatchObject({
			status: "open",
			commitment: "0xa2",
			entryPoint: ENTRY_POINT_V09,
			orderNonce: "1",
		})
		await second.close()
	})
})
