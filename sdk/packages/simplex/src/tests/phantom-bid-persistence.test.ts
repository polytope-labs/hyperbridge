import type { HexString } from "@hyperbridge/sdk"
import { describe, expect, it } from "vitest"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { patchRuntimeState } from "@/data/state"
import { stubOrderScanner } from "./helpers/stub-scanner"

const COMMITMENT = "0x336556333c8ee4f15a4688090eb237443f36c9e3d3270690649549a1f8009a92" as HexString

function filler(data: MemoryDataStore): IntentFiller {
	// The constructor only reads the config service for chain and endpoint lookups.
	const configService = {
		getConfiguredChainIds: () => [8453],
		getEntryPointAddress: () => undefined,
		getHyperbridgeWsUrl: () => undefined,
		getSubstratePrivateKey: () => undefined,
	} as never
	return new IntentFiller(
		[],
		[],
		{ maxConcurrentOrders: 1 } as never,
		configService,
		{} as never,
		{} as never,
		{ address: "0xAAAA00000000000000000000000000000000AAAA" as HexString } as never,
		{ orders: stubOrderScanner() },
		undefined,
		data.bids,
		data.state,
	)
}

describe("phantom bid persistence", () => {
	it("persists the live phantom bid per chain and restores it on the next run", async () => {
		const data = new MemoryDataStore()
		const first = filler(data)
		// The private bookkeeping the batch path calls once a bid lands or is pooled.
		;(first as unknown as { rememberPhantomBid: (chain: string, commitment: HexString) => void }).rememberPhantomBid(
			"EVM-8453",
			COMMITMENT,
		)
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect((await data.state.get()).phantomBids).toEqual({ "EVM-8453": COMMITMENT })

		const second = filler(data)
		second.restorePhantomBids((await data.state.get()).phantomBids)
		expect(second.livePhantomBids()).toEqual({ "EVM-8453": COMMITMENT })
	})

	it("does not overwrite a commitment this run already knows", () => {
		const data = new MemoryDataStore()
		const current = filler(data)
		;(current as unknown as { rememberPhantomBid: (chain: string, commitment: HexString) => void }).rememberPhantomBid(
			"EVM-8453",
			COMMITMENT,
		)
		current.restorePhantomBids({ "EVM-8453": "0xstale", "EVM-1": "0xother" })
		expect(current.livePhantomBids()).toEqual({ "EVM-8453": COMMITMENT, "EVM-1": "0xother" })
	})

	it("pause and resume keep the phantom bids in the state record", async () => {
		const data = new MemoryDataStore()
		await patchRuntimeState(data.state, { phantomBids: { "EVM-8453": COMMITMENT } })
		await patchRuntimeState(data.state, { paused: true })
		expect(await data.state.get()).toEqual({ paused: true, phantomBids: { "EVM-8453": COMMITMENT } })
		await patchRuntimeState(data.state, { paused: false })
		expect((await data.state.get()).phantomBids).toEqual({ "EVM-8453": COMMITMENT })
	})
})
