import { describe, expect, it } from "vitest"
import { chainsForNetwork, INIT_CHAINS } from "@/cli/init/chains"
import { validateRpcUrls } from "@/services/FillerConfigService"
import { quorumThreshold } from "@/services/QuorumPublicClient"

/**
 * The public endpoint sets both wizards start from.
 *
 * They exist so an operator can run without an RPC account, which only holds if
 * the list is actually usable as written: every chain's set has to clear the
 * filler's own validation and leave a quorum that can survive a provider having
 * a bad day. These pin the properties that made the list what it is, because
 * the tempting edit — pasting in another URL from a provider already on the
 * list — breaks the one that is invisible.
 */
describe("bundled public RPC endpoints", () => {
	const mainnet = chainsForNetwork("mainnet")

	it("covers every mainnet chain the wizards offer", () => {
		for (const meta of mainnet) {
			expect(meta.defaultRpcUrls?.length, `${meta.label} has no bundled endpoints`).toBeGreaterThan(0)
		}
	})

	it("leaves testnets alone, where nothing was measured", () => {
		for (const meta of INIT_CHAINS.filter((c) => c.network === "testnet")) {
			expect(meta.defaultRpcUrls, `${meta.label} should have no bundled endpoints`).toBeUndefined()
		}
	})

	it("passes the filler's own endpoint validation", () => {
		for (const meta of mainnet) {
			expect(() => validateRpcUrls(meta.defaultRpcUrls ?? []), meta.label).not.toThrow()
		}
	})

	it("carries no API keys, which would be a shared quota", () => {
		for (const meta of mainnet) {
			for (const url of meta.defaultRpcUrls ?? []) {
				expect(url, `${meta.label}: ${url}`).not.toMatch(/api[-_]?key|apikey|\/v2\/[0-9a-zA-Z_-]{20,}/i)
			}
		}
	})

	it("lists one endpoint per operator, so no two voters are the same opinion", () => {
		// Registrable domain, not hostname: `gateway.tenderly.co` and
		// `polygon.gateway.tenderly.co` are distinct hostnames and one provider,
		// which is exactly the case `validateRpcUrls` cannot see.
		for (const meta of mainnet) {
			const operators = (meta.defaultRpcUrls ?? []).map((url) =>
				new URL(url).hostname.split(".").slice(-2).join("."),
			)
			expect(new Set(operators).size, `${meta.label} repeats an operator: ${operators.join(", ")}`).toBe(
				operators.length,
			)
		}
	})

	it("leaves room for an endpoint to drop out without failing the chain", () => {
		for (const meta of mainnet) {
			const total = meta.defaultRpcUrls?.length ?? 0
			// A BFT bar tolerates floor((n-1)/3) faults; below one the set is only
			// as good as its flakiest member, which free endpoints regularly are.
			expect(total - quorumThreshold(total), meta.label).toBeGreaterThanOrEqual(1)
		}
	})
})
