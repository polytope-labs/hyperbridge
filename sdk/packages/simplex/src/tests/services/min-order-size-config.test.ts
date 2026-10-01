import { describe, expect, it } from "vitest"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { FillerConfigService } from "@/services/FillerConfigService"

/**
 * `minOrderSizeUsd` is the order size below which `order.fees` must cover the execution
 * cost. An order at or above it is filled whatever fees it carries.
 */

const CHAINS = [{ chainId: 8453, rpcUrls: ["https://rpc.example"] }] as any

function service(fillerConfig?: Record<string, unknown>) {
	return new FillerConfigService(CHAINS, fillerConfig as any)
}

function toml(minOrderSizeUsd?: number): FillerTomlConfig {
	return {
		orderbook: { url: "https://orderbook.example/graphql" },
		simplex: {
			maxConcurrentOrders: 5,
			substratePrivateKey: "seed phrase here",
			hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
			minOrderSizeUsd,
		},
		pairs: [{ token0: "USDC", token1: "USDC" }],
		chains: [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }],
	}
}

describe("minOrderSizeUsd", () => {
	it("defaults to $20 when the operator does not set it", () => {
		expect(service().getMinOrderSizeUsd()).toBe(20)
	})

	it("takes the configured value when set", () => {
		expect(service({ minOrderSizeUsd: 250 }).getMinOrderSizeUsd()).toBe(250)
	})

	it("is honoured at zero, which checks no order's fees", () => {
		expect(service({ minOrderSizeUsd: 0 }).getMinOrderSizeUsd()).toBe(0)
	})

	it("is accepted unset, at zero and as a fraction of a dollar", () => {
		for (const value of [undefined, 0, 0.5, 20]) {
			expect(() => validateConfig(toml(value))).not.toThrow()
		}
	})

	it("is rejected when negative or not a number", () => {
		for (const bad of [-1, Number.NaN, Number.POSITIVE_INFINITY]) {
			expect(() => validateConfig(toml(bad))).toThrow(/minOrderSizeUsd/)
		}
	})
})
