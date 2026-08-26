import { describe, it, expect, vi } from "vitest"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { CacheService } from "@/services/CacheService"
import type { Signer } from "@/services/wallet"

const CHAIN = "EVM-137"
/** A 6-decimal token, i.e. one where guessing 18 overstates the payout by 10^12. */
const USDC = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"
const UNKNOWN = "0xdEaDbEeF00000000000000000000000000000000"
const NATIVE = "0x0000000000000000000000000000000000000000"

/**
 * Builds a service whose on-chain `decimals()` either resolves or always throws,
 * with a registry that knows about USDC only.
 */
function makeService(opts: { onChain?: number; registry?: number; chainIds?: number[] }) {
	const readContract = opts.onChain === undefined
		? vi.fn().mockRejectedValue(new Error("RPC returned undecodable data"))
		: vi.fn().mockResolvedValue(opts.onChain)

	const clientManager = { getPublicClient: vi.fn().mockReturnValue({ readContract }) } as unknown as ChainClientManager

	const getAssetDecimalsByAddress = vi.fn((_chain: string, address: string) =>
		address.toLowerCase() === USDC.toLowerCase() ? opts.registry : undefined,
	)

	const configService = {
		getAssetDecimalsByAddress,
		// The constructor kicks off initCache() without awaiting it.
		getConfiguredChainIds: vi.fn().mockReturnValue(opts.chainIds ?? []),
		getUsdcAsset: vi.fn().mockReturnValue(USDC),
		getUsdtAsset: vi.fn().mockReturnValue(UNKNOWN),
		getHostAddress: vi.fn().mockReturnValue("0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3"),
	} as unknown as FillerConfigService

	const setTokenDecimals = vi.fn()
	const cacheService = {
		getTokenDecimals: vi.fn().mockReturnValue(undefined),
		setTokenDecimals,
	} as unknown as CacheService

	const signer = { address: "0x1111111111111111111111111111111111111111" } as unknown as Signer

	const service = new ContractInteractionService(clientManager, configService, signer, cacheService)
	return { service, readContract, getAssetDecimalsByAddress, setTokenDecimals }
}

describe("ContractInteractionService.getTokenDecimals", () => {
	it("uses the on-chain value and caches it, without consulting the registry", async () => {
		const { service, getAssetDecimalsByAddress, setTokenDecimals } = makeService({ onChain: 6, registry: 6 })

		await expect(service.getTokenDecimals(USDC, CHAIN)).resolves.toBe(6)

		expect(getAssetDecimalsByAddress).not.toHaveBeenCalled()
		expect(setTokenDecimals).toHaveBeenCalledWith(CHAIN, USDC, 6)
	})

	it("falls back to the configured decimals when the on-chain read fails", async () => {
		const { service, getAssetDecimalsByAddress } = makeService({ registry: 6 })

		// Regression: this used to return a hardcoded 18. `computeLegPolicyOutput`
		// scales policyMaxOutput by 10 ** decimals, so 18-for-6 inflates the payout
		// by 10^12 — and with the overfill clamp disabled nothing bounds it back to
		// the user's requested output.
		await expect(service.getTokenDecimals(USDC, CHAIN)).resolves.toBe(6)

		expect(getAssetDecimalsByAddress).toHaveBeenCalledWith(CHAIN, USDC)
	})

	it("does not cache the fallback, so a later call retries the authoritative read", async () => {
		const { service, setTokenDecimals } = makeService({ registry: 6 })

		await service.getTokenDecimals(USDC, CHAIN)

		// CacheService.tokenDecimals has no TTL: caching a registry value during an
		// RPC outage would pin it for the lifetime of the process.
		expect(setTokenDecimals).not.toHaveBeenCalled()
	})

	it("throws when the read fails and the token is not in the registry", async () => {
		const { service, getAssetDecimalsByAddress } = makeService({})

		// Never guess: decimals scales policyMaxOutput by 10 ** decimals, so a wrong
		// value changes the fill size by orders of magnitude rather than degrading it.
		// Callers on the fill path treat a throw as "skip this order".
		await expect(service.getTokenDecimals(UNKNOWN, CHAIN)).rejects.toThrow(/Unable to determine decimals/)

		expect(getAssetDecimalsByAddress).toHaveBeenCalledWith(CHAIN, UNKNOWN)
	})

	it("prewarming never rejects, so an unawaited initCache cannot kill the process", async () => {
		// A configured chain whose every read fails and whose USDT entry is absent from
		// the registry — i.e. the prewarm hits the throwing branch on both tokens.
		const { service } = makeService({ chainIds: [137] })

		// initCache() runs unawaited from the constructor; a rejection there would be an
		// unhandled rejection rather than a skipped order.
		await expect(service.initCache()).resolves.toBeUndefined()
	})

	it("short-circuits the native token without reading or consulting the registry", async () => {
		const { service, readContract, getAssetDecimalsByAddress } = makeService({ onChain: 6 })

		await expect(service.getTokenDecimals(NATIVE, CHAIN)).resolves.toBe(18)

		expect(readContract).not.toHaveBeenCalled()
		expect(getAssetDecimalsByAddress).not.toHaveBeenCalled()
	})
})
