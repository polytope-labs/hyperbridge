import { describe, expect, it } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import type { SetupDefaults } from "../types"
import { assembleConfig, initialState } from "./state"

/** The wizard has no bundler field: a chain's bundler is Hyperbridge's, written without asking. */
describe("wizard bundlers", () => {
	const defaults = {
		chains: chainsForNetwork("mainnet"),
		hyperbridgeWs: { mainnet: "wss://nexus.rpc.polytope.technology" },
		usdStables: ["USDC", "USDT"],
		maxConcurrentOrders: 5,
		configPath: "/tmp/filler-config.toml",
		knownTokens: {},
		knownVaults: {},
	} as unknown as SetupDefaults

	it("keeps no bundler in the chain drafts", () => {
		for (const chain of initialState(defaults).chains) expect(chain).not.toHaveProperty("bundlerUrl")
	})

	it("writes Hyperbridge's bundler for every enabled chain", () => {
		const state = initialState(defaults)
		const enabled = new Set([1, 8453, 56])
		const config = assembleConfig(
			{ ...state, chains: state.chains.map((chain) => ({ ...chain, enabled: enabled.has(chain.meta.chainId) })) },
			defaults,
		)
		expect(config.chains.map((chain) => chain.bundlerUrl).sort()).toEqual([
			"https://bundler.polytope.technology/base",
			"https://bundler.polytope.technology/bsc",
			"https://bundler.polytope.technology/ethereum",
		])
	})
})
