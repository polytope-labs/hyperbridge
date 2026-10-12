import { describe, expect, it } from "vitest"
import { EvmChain } from "@/chains/evm"

const host = "0x9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1"
const rpcUrl = "http://127.0.0.1:8545"

describe("EvmChain.bundlerUrl", () => {
	it("defaults to the bundler Hyperbridge runs for the chain", () => {
		expect(EvmChain.fromParams({ chainId: 8453, rpcUrl, host }).bundlerUrl).toBe(
			"https://bundler.polytope.technology/base",
		)
		expect(EvmChain.fromParams({ chainId: 97, rpcUrl, host }).bundlerUrl).toBe(
			"https://bundler.polytope.technology/bsc-chapel",
		)
	})

	it("keeps a bundler it is given", () => {
		const bundlerUrl = "https://bundler.example.com"
		expect(EvmChain.fromParams({ chainId: 8453, rpcUrl, host, bundlerUrl }).bundlerUrl).toBe(bundlerUrl)
	})

	it("is undefined on a chain Hyperbridge runs no bundler for", () => {
		expect(EvmChain.fromParams({ chainId: 11155111, rpcUrl, host }).bundlerUrl).toBeUndefined()
	})
})
