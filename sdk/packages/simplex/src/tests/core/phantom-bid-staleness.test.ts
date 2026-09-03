import { stubOrderScanner } from "../helpers/stub-scanner"
import { describe, expect, it, vi } from "vitest"
import type { HexString, PhantomOrderEvent } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"

/**
 * A phantom bid is only worth anything inside its order's window. Everything upstream of the bid
 * can add delay — a poll cursor behind the head, the global queue, quoting — and none of it shows
 * up in the event, which carries the block it was registered at and no clock.
 *
 * Bidding late is not a harmless no-op: the extrinsic is accepted and reserves a deposit, while the
 * aggregation read that order's bids when its window closed, so nothing counts it. On mainnet a
 * filler sat 3,448 blocks behind for nine hours in exactly this state — bids landing, no errors,
 * and backing no pool at all.
 */

const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const CHAIN = "EVM-8453"
const CHAIN_ID = 8453

const event = (createdAt: number, commitment: string): PhantomOrderEvent => ({
	commitment: commitment as HexString,
	chain: CHAIN,
	createdAt,
	legs: [],
})

const FRESH = `0x${"11".repeat(32)}`
const STALE = `0x${"22".repeat(32)}`

function build(head: number | Error) {
	const configService = {
		getConfiguredChainIds: () => [CHAIN_ID],
		getEntryPointAddress: () => ENTRY_POINT,
		getHyperbridgeWsUrl: () => undefined,
		getSubstratePrivateKey: () => undefined,
	} as any

	const filler = new IntentFiller(
		[],
		[],
		{ maxConcurrentOrders: 1, watchOnly: {} } as any,
		configService,
		{} as any, // ChainClientManager — an order that survives the age gate stops at the fetch below
		{} as any, // ContractInteractionService — likewise
		{ address: "0xAAAA00000000000000000000000000000000AAAA" as HexString } as any,
		{ orders: stubOrderScanner() },
	)

	// Returning null ends preparation right after the fetch, so the call itself is the signal that
	// the age gate let an order through.
	const fetchPhantomOrder = vi.fn(async () => null)
	const latestBlockNumber = vi.fn(async () => {
		if (head instanceof Error) throw head
		return head
	})
	const submitPhantomBids = vi.fn()
	const coprocessor = { fetchPhantomOrder, latestBlockNumber, submitPhantomBids } as any

	const handle = (events: PhantomOrderEvent[]) => (filler as any).handlePhantomOrders(events, coprocessor)

	return { handle, fetchPhantomOrder, submitPhantomBids }
}

describe("phantom order staleness", () => {
	it("does not bid on an order whose window has closed", async () => {
		const { handle, fetchPhantomOrder, submitPhantomBids } = build(1_000)

		await handle([event(900, STALE)])

		expect(fetchPhantomOrder).not.toHaveBeenCalled()
		// Nothing is submitted, so no deposit is reserved for a bid nobody can count.
		expect(submitPhantomBids).not.toHaveBeenCalled()
	})

	it("still bids on the orders of the same batch that are live", async () => {
		const { handle, fetchPhantomOrder } = build(1_000)

		await handle([event(900, STALE), event(995, FRESH)])

		expect(fetchPhantomOrder).toHaveBeenCalledTimes(1)
		expect(fetchPhantomOrder).toHaveBeenCalledWith(FRESH)
	})

	// A bid a few blocks old is the normal case — the poll, the queue and the quoting all cost
	// blocks — so the gate must not be so tight that it throws away every bid this filler places.
	it("bids on an order that is merely a few blocks old", async () => {
		const { handle, fetchPhantomOrder } = build(1_000)

		await handle([event(990, FRESH)])

		expect(fetchPhantomOrder).toHaveBeenCalledTimes(1)
	})

	// One flaky endpoint must not be able to stop this filler bidding altogether: without a head
	// there is no evidence the order is stale, and the old behaviour (bid anyway) is the safer of
	// the two failures.
	it("bids on everything when the head cannot be read", async () => {
		const { handle, fetchPhantomOrder } = build(new Error("rpc unavailable"))

		await handle([event(1, STALE), event(2, FRESH)])

		expect(fetchPhantomOrder).toHaveBeenCalledTimes(2)
	})
})
