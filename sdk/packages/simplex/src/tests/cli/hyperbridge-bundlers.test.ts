import { describe, expect, it, vi } from "vitest"
import { chainsForNetwork, INIT_CHAINS } from "@/cli/init/chains"
import type { WizardState } from "@/cli/init/state"
import { stepBundlers } from "@/cli/init/steps/bundlers"

vi.mock("@clack/prompts", () => {
	const asked = () => {
		throw new Error("the wizard asked a question")
	}
	return {
		confirm: asked,
		text: asked,
		password: asked,
		select: asked,
		note: asked,
		log: { info: asked, message: asked },
	}
})

/**
 * The bundler the wizards and the Chains panel use for a chain, never asking the operator for
 * one. Every chain they offer on mainnet has to carry it, or the config they write names no
 * bundler for that chain and does not validate.
 */
describe("Hyperbridge bundlers", () => {
	const mainnet = chainsForNetwork("mainnet")

	it("covers every mainnet chain the wizards offer", () => {
		expect(Object.fromEntries(mainnet.map((meta) => [meta.chainId, meta.hyperbridgeBundlerUrl]))).toEqual({
			1: "https://bundler.polytope.technology/ethereum",
			56: "https://bundler.polytope.technology/bsc",
			137: "https://bundler.polytope.technology/polygon",
			8453: "https://bundler.polytope.technology/base",
			42161: "https://bundler.polytope.technology/arbitrum",
		})
	})

	it("names none for testnets, which take their bundler from the config", () => {
		for (const meta of INIT_CHAINS.filter((chain) => chain.network === "testnet")) {
			expect(meta.hyperbridgeBundlerUrl, meta.label).toBeUndefined()
		}
	})

	it("is what the terminal wizard writes, without a question and over an earlier choice", async () => {
		const state = {
			chains: mainnet.map((meta, index) => ({
				meta,
				rpcUrls: ["https://rpc.example"],
				bundlerUrl: index === 0 ? "https://another-bundler.example" : undefined,
			})),
		} as unknown as WizardState

		await stepBundlers(state)

		expect(state.chains.map((chain) => chain.bundlerUrl)).toEqual(mainnet.map((meta) => meta.hyperbridgeBundlerUrl))
	})
})
