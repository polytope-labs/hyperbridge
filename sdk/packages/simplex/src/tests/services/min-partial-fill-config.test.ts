import { describe, expect, it } from "vitest"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { FillerConfigService } from "@/services/FillerConfigService"

/**
 * `minPartialFillUsd` is the payout below which a partial fill is not bid. A
 * partial collects none of the order's fees and is exempt from the profit floor,
 * so without it a near-exhausted limit order bids dust that pays its own gas.
 */

const CHAINS = [{ chainId: 8453, rpcUrls: ["https://rpc.example"] }] as any

function service(fillerConfig?: Record<string, unknown>) {
	return new FillerConfigService(CHAINS, fillerConfig as any)
}

function toml(minPartialFillUsd?: number): FillerTomlConfig {
	return {
		orderbook: { url: "https://orderbook.example/graphql" },
		simplex: {
			maxConcurrentOrders: 5,
			substratePrivateKey: "seed phrase here",
			hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
			minPartialFillUsd,
		},
		chains: [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }],
	}
}

describe("minPartialFillUsd", () => {
	it("defaults to $20 when the operator does not set it", () => {
		expect(service().getMinPartialFillUsd()).toBe(20)
	})

	it("takes the configured value when set", () => {
		expect(service({ minPartialFillUsd: 250 }).getMinPartialFillUsd()).toBe(250)
	})

	it("is honoured at zero, which disables the floor", () => {
		expect(service({ minPartialFillUsd: 0 }).getMinPartialFillUsd()).toBe(0)
	})

	it("is accepted unset, at zero and as a fraction of a dollar", () => {
		for (const value of [undefined, 0, 0.5, 20]) {
			expect(() => validateConfig(toml(value))).not.toThrow()
		}
	})

	it("is rejected when negative or not a number", () => {
		for (const bad of [-1, Number.NaN, Number.POSITIVE_INFINITY]) {
			expect(() => validateConfig(toml(bad))).toThrow(/minPartialFillUsd/)
		}
	})
})
