import { describe, it, expect } from "vitest"
import { PUBLIC_RPC_URLS, getPublicRpcUrls } from "@/config/public-rpcs"
import { FillerConfigService, type ResolvedChainConfig } from "@/services/FillerConfigService"
import { QuorumPublicClient } from "@/services/QuorumPublicClient"

// Unit tests for the public RPC registry and how FillerConfigService merges it
// into the quorum provider set. No network access required.

const SUPPORTED_CHAIN_IDS = [1, 56, 137, 8453, 42161]

describe("PUBLIC_RPC_URLS registry", () => {
	it("covers every supported EVM mainnet", () => {
		for (const chainId of SUPPORTED_CHAIN_IDS) {
			expect(PUBLIC_RPC_URLS[chainId], `chain ${chainId}`).toBeDefined()
			expect(PUBLIC_RPC_URLS[chainId].length).toBeGreaterThanOrEqual(3)
		}
	})

	it("lists only well-formed https URLs with distinct hostnames per chain", () => {
		for (const [chainId, urls] of Object.entries(PUBLIC_RPC_URLS)) {
			const hosts = new Set<string>()
			for (const url of urls) {
				const parsed = new URL(url)
				expect(parsed.protocol, `${chainId}: ${url}`).toBe("https:")
				expect(hosts.has(parsed.hostname), `${chainId}: duplicate host ${parsed.hostname}`).toBe(false)
				hosts.add(parsed.hostname)
			}
		}
	})

	it("returns an empty list for chains without a registry entry", () => {
		expect(getPublicRpcUrls(130)).toEqual([])
		expect(getPublicRpcUrls(11155111)).toEqual([])
	})
})

describe("FillerConfigService.getQuorumRpcUrls", () => {
	function makeService(chainId: number, rpcUrls: string[]): FillerConfigService {
		const chains: ResolvedChainConfig[] = [{ chainId, rpcUrls }]
		return new FillerConfigService(chains)
	}

	it("appends the public registry after the operator's endpoints", () => {
		const userUrls = ["https://my-premium-node.example.com"]
		const service = makeService(8453, userUrls)

		const merged = service.getQuorumRpcUrls("EVM-8453")

		// Operator endpoints come first, unchanged.
		expect(merged[0]).toBe(userUrls[0])
		// Every public endpoint for the chain is appended.
		for (const url of getPublicRpcUrls(8453)) {
			expect(merged).toContain(url)
		}
		expect(merged).toHaveLength(userUrls.length + getPublicRpcUrls(8453).length)
	})

	it("dedupes by hostname with operator endpoints taking precedence", () => {
		// The operator already uses one of the public endpoints.
		const userUrls = ["https://my-premium-node.example.com", "https://mainnet.base.org"]
		const service = makeService(8453, userUrls)

		const merged = service.getQuorumRpcUrls("EVM-8453")

		expect(merged.slice(0, 2)).toEqual(userUrls)
		expect(merged.filter((url) => new URL(url).hostname === "mainnet.base.org")).toHaveLength(1)
		expect(merged).toHaveLength(userUrls.length + getPublicRpcUrls(8453).length - 1)
	})

	it("returns operator endpoints unchanged for chains without a registry entry", () => {
		const userUrls = ["https://unichain.example.com"]
		const service = makeService(130, userUrls)
		expect(service.getQuorumRpcUrls("EVM-130")).toEqual(userUrls)
	})

	it("produces a set the QuorumPublicClient accepts (distinct hostnames)", () => {
		const service = makeService(8453, ["https://my-premium-node.example.com", "https://mainnet.base.org"])
		const merged = service.getQuorumRpcUrls("EVM-8453")

		const client = new QuorumPublicClient(8453, merged)
		expect(client.size).toBe(merged.length)
		// 5 providers → BFT threshold 4: one faulty public endpoint is tolerated.
		expect(client.threshold).toBe(Math.floor((2 * merged.length) / 3) + 1)
	})
})
