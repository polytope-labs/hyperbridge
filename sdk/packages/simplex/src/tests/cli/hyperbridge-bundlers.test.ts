import { chainConfigs } from "@hyperbridge/sdk"
import { describe, expect, it } from "vitest"
import { chainsForNetwork, INIT_CHAINS } from "@/cli/init/chains"
import { HYPERBRIDGE_BUNDLER_URLS, hyperbridgeBundlerUrl } from "@/config/bundlers"

/**
 * Simplex fills through Hyperbridge's bundler for each chain and takes none from its config, so
 * the wizards and the Chains panel only offer chains that have one.
 */
describe("Hyperbridge bundlers", () => {
	it("are the bundlers Hyperbridge runs", () => {
		expect(HYPERBRIDGE_BUNDLER_URLS).toEqual({
			1: "https://bundler.polytope.technology/ethereum",
			56: "https://bundler.polytope.technology/bsc",
			97: "https://bundler.polytope.technology/bsc-chapel",
			137: "https://bundler.polytope.technology/polygon",
			8453: "https://bundler.polytope.technology/base",
			42161: "https://bundler.polytope.technology/arbitrum",
			80002: "https://bundler.polytope.technology/polygon-amoy",
		})
		expect(hyperbridgeBundlerUrl(11155111)).toBeUndefined()
	})

	it("are the ones the SDK defaults to", () => {
		const sdk = Object.fromEntries(
			Object.values(chainConfigs)
				.filter((config) => config.bundlerUrl)
				.map((config) => [config.chainId, config.bundlerUrl]),
		)
		expect(sdk).toEqual(HYPERBRIDGE_BUNDLER_URLS)
	})

	it("cover every chain the wizards offer, on both networks", () => {
		for (const network of ["mainnet", "testnet"] as const) {
			for (const meta of chainsForNetwork(network)) {
				expect(meta.hyperbridgeBundlerUrl, meta.label).toBe(HYPERBRIDGE_BUNDLER_URLS[meta.chainId])
			}
		}
		expect(chainsForNetwork("testnet").map((meta) => meta.chainId)).toEqual([80002, 97])
	})

	it("leave chains without one out of the wizards", () => {
		const offered = new Set([...chainsForNetwork("mainnet"), ...chainsForNetwork("testnet")].map((m) => m.chainId))
		for (const meta of INIT_CHAINS.filter((chain) => !chain.hyperbridgeBundlerUrl)) {
			expect(offered.has(meta.chainId), meta.label).toBe(false)
		}
	})
})
