import { describe, expect, it } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import { validateConfig } from "@/config/filler-toml"
import { DEFAULT_ORDERBOOK_URLS } from "@/config/defaults"
import type { SetupDefaults } from "../types"
import { assembleConfig, initialState, orderbookPairs, type WizardState } from "./state"

/**
 * The wizard has no markets step: a config declares one market per book the
 * orderbook lists, and limit orders price them. A config with no markets would
 * boot no trading engine at all, so the pairs are not optional.
 */
describe("wizard markets from the orderbook", () => {
	const USDC_BASE = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
	const CNGN_BASE = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F"
	const USDT_BSC = "0x55d398326f99059fF775485246999027B3197955"

	const defaults = {
		chains: chainsForNetwork("mainnet"),
		hyperbridgeWs: { mainnet: "wss://nexus.rpc.polytope.technology" },
		usdStables: ["USDC", "USDT"],
		maxConcurrentOrders: 5,
		configPath: "/tmp/filler-config.toml",
		knownTokens: {
			"EVM-8453": [
				{ symbol: "USDC", address: USDC_BASE },
				{ symbol: "cNGN", address: CNGN_BASE },
			],
			"EVM-56": [{ symbol: "USDT", address: USDT_BSC }],
		},
		knownVaults: {},
	} as unknown as SetupDefaults

	const books = [
		{ id: "USDC-cNGN", base: "USDC", quote: "cNGN" },
		{ id: "USDT-cNGN", base: "USDT", quote: "cNGN" },
	]

	/** A wizard that has read the orderbook and enabled the given chains, fully configured. */
	function wizardWith(chainIds: number[]): WizardState {
		const state = initialState(defaults)
		return {
			...state,
			signerKey: `0x${"11".repeat(32)}`,
			substrateKey: `0x${"22".repeat(32)}`,
			orderbook: { url: DEFAULT_ORDERBOOK_URLS.mainnet, books },
			chains: state.chains.map((chain) =>
				chainIds.includes(chain.meta.chainId)
					? { ...chain, enabled: true, bundlerUrl: `https://bundler.example/${chain.meta.chainId}` }
					: chain,
			),
		}
	}

	it("declares every book whose assets are deployed on an enabled chain", () => {
		expect(orderbookPairs(wizardWith([8453]), defaults)).toEqual([{ token0: "USDC", token1: "cNGN" }])
	})

	it("counts an asset on any enabled chain, as boot does, not only the book's own chain", () => {
		// USDT lives on BSC here and cNGN on Base: the cross-chain market still resolves.
		expect(orderbookPairs(wizardWith([8453, 56]), defaults)).toEqual([
			{ token0: "USDC", token1: "cNGN" },
			{ token0: "USDT", token1: "cNGN" },
		])
	})

	it("declares nothing when the enabled chains carry no book", () => {
		expect(orderbookPairs(wizardWith([1]), defaults)).toEqual([])
	})

	it("writes the orderbook section the config cannot boot without", () => {
		const config = assembleConfig(wizardWith([8453]), defaults)
		expect(config.orderbook).toEqual({ url: DEFAULT_ORDERBOOK_URLS.mainnet })
		// The check that used to stop the wizard: "an [orderbook] section is required".
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("still names the mainnet orderbook before the books have been read", () => {
		const unread = { ...wizardWith([8453]), orderbook: undefined }
		expect(assembleConfig(unread, defaults).orderbook).toEqual({ url: DEFAULT_ORDERBOOK_URLS.mainnet })
	})
})
