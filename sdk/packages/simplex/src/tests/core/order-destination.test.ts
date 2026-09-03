import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * An order's destination decides where the fill would execute. A destination
 * this filler has no configuration for — or watches without filling — is
 * expected mainnet traffic (other fillers cover other lanes), and used to fall
 * through to the solver-selection cache check, which dropped the order with a
 * misleading "Shared cache is not initialized" ERROR: the cache is only ever
 * populated for configured, non-watch-only chains, so the error fired for
 * healthy fillers on every cross-lane order.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString

function build(watchOnly: Record<number, boolean> = {}) {
	const getSolverSelection = vi.fn((chain: string) => (chain === "EVM-1" ? false : null))
	const isUserAllowed = vi.fn(() => false) // ends the happy path right after the guards

	const filler = new IntentFiller(
		[{ chainId: 1 } as never],
		[],
		{ maxConcurrentOrders: 1, watchOnly } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getConfiguredChainIds: () => [1, 2],
			isUserAllowed,
		} as never,
		{} as never,
		{ getCache: () => ({ getSolverSelection }) } as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([1]) },
		undefined,
		new MemoryDataStore().bids,
	)
	return { filler, getSolverSelection, isUserAllowed }
}

async function detect(filler: IntentFiller, destination: string) {
	;(filler as any).handleNewOrder({ id: "0xorder", user: "0xUSER", source: "EVM-1", destination }, "0xtx")
	await (filler as any).globalQueue.onIdle()
}

describe("order destination gating", () => {
	it("skips an order destined to an unconfigured chain without touching the cache", async () => {
		const { filler, getSolverSelection } = build()
		await detect(filler, "EVM-999999")
		expect(getSolverSelection).not.toHaveBeenCalled()
	})

	it("skips a destination that is not a chain key at all", async () => {
		const { filler, getSolverSelection } = build()
		await detect(filler, "KUSAMA-4009")
		expect(getSolverSelection).not.toHaveBeenCalled()
	})

	it("skips an order destined to a watch-only chain", async () => {
		const { filler, getSolverSelection } = build({ 2: true })
		await detect(filler, "EVM-2")
		expect(getSolverSelection).not.toHaveBeenCalled()
	})

	it("a configured, filling destination proceeds through the cache to the allowlist", async () => {
		const { filler, getSolverSelection, isUserAllowed } = build()
		await detect(filler, "EVM-1")
		expect(getSolverSelection).toHaveBeenCalledWith("EVM-1")
		expect(isUserAllowed).toHaveBeenCalled()
	})

	// The error is reserved for what it actually means now: a destination this
	// filler is configured to fill on whose cache entry is genuinely absent.
	it("still errors on a configured destination with no cache entry", async () => {
		const { filler, getSolverSelection, isUserAllowed } = build()
		await detect(filler, "EVM-2") // configured (id 2), not watch-only, cache returns null
		expect(getSolverSelection).toHaveBeenCalledWith("EVM-2")
		expect(isUserAllowed).not.toHaveBeenCalled()
	})
})
