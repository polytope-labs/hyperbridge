import { stubOrderStream } from "../helpers/stub-stream"
import { describe, expect, it, vi } from "vitest"
import type { HexString, PhantomOrderEvent } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"

/**
 * The pallet registers one phantom order per chain it knows about — a superset of what any single
 * operator runs. Nothing downstream narrows that set on its own: the strategies price legs off the
 * shared asset registry, not the operator's config, so an unconfigured chain quotes happily and the
 * filler publishes a bid it could never honour (it has no RPC or bundler for that chain, so the
 * fill path never even sees the real order). Watch-only chains are the same story by config.
 */

const COMMITMENT = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const CONFIGURED_CHAIN_ID = 8453 // Base — the only chain in the operator's config below

function event(chain: string): PhantomOrderEvent {
	return { commitment: COMMITMENT, chain, createdAt: 1, legs: [] }
}

function build(watchOnly: Record<number, boolean> = {}) {
	const configService = {
		getConfiguredChainIds: () => [CONFIGURED_CHAIN_ID],
		getEntryPointAddress: () => ENTRY_POINT,
		getHyperbridgeWsUrl: () => undefined,
		getSubstratePrivateKey: () => undefined,
	} as any

	const filler = new IntentFiller(
		[],
		[],
		{ maxConcurrentOrders: 1, watchOnly } as any,
		configService,
		{} as any, // ChainClientManager — the gate returns before any client is touched
		{} as any, // ContractInteractionService — likewise
		{ account: { address: "0xAAAA00000000000000000000000000000000AAAA" as HexString } } as any,
		{ orders: stubOrderStream() },
	)

	// Returning null stops the handler right after the fetch, so the call itself is the signal
	// that the chain gate let the order through.
	const fetchPhantomOrder = vi.fn(async () => null)
	const coprocessor = { fetchPhantomOrder } as any

	const handle = (chain: string) => (filler as any).preparePhantomBid(event(chain), coprocessor)

	return { handle, fetchPhantomOrder }
}

describe("phantom bidding chain gating", () => {
	it("skips a chain absent from the config", async () => {
		const { handle, fetchPhantomOrder } = build()

		await handle("EVM-56") // BSC: a real chain in the registry, just not one this operator runs

		expect(fetchPhantomOrder).not.toHaveBeenCalled()
	})

	it("skips a chain the registry does not know", async () => {
		const { handle, fetchPhantomOrder } = build()

		await handle("EVM-999999")

		expect(fetchPhantomOrder).not.toHaveBeenCalled()
	})

	it("skips a configured chain that is watch-only", async () => {
		const { handle, fetchPhantomOrder } = build({ [CONFIGURED_CHAIN_ID]: true })

		await handle("EVM-8453")

		expect(fetchPhantomOrder).not.toHaveBeenCalled()
	})

	it("quotes a configured chain", async () => {
		const { handle, fetchPhantomOrder } = build()

		await handle("EVM-8453")

		expect(fetchPhantomOrder).toHaveBeenCalledWith(COMMITMENT)
	})
})
