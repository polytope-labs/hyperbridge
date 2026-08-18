import { describe, it, expect, vi, beforeEach } from "vitest"
import { fallback, http } from "viem"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { CacheService } from "@/services/CacheService"
import type { Signer } from "@/services/wallet"

const HOST = "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3"
const PRIMARY = "https://polygon-bor-rpc.publicnode.com"
const SECONDARY = "https://gateway.tenderly.co/public/polygon"

const fromParams = vi.hoisted(() => vi.fn((params: unknown) => params))
const create = vi.hoisted(() => vi.fn(async () => ({}) as never))

vi.mock("@hyperbridge/sdk", async (importOriginal) => ({
	...(await importOriginal<Record<string, unknown>>()),
	EvmChain: { fromParams },
	IntentGateway: { create },
}))

/**
 * A multi-URL chain is served by a viem `fallback` transport, whose `url` is
 * undefined — the shape that used to leak through to the SDK.
 */
function makeService(rpcUrls: string[]) {
	const transport = rpcUrls.length === 1 ? http(rpcUrls[0]) : fallback(rpcUrls.map((u) => http(u)))
	const publicClient = { transport: transport({ chain: undefined } as never) }

	const clientManager = { getPublicClient: vi.fn().mockReturnValue(publicClient) } as unknown as ChainClientManager
	const configService = {
		getRpcUrls: vi.fn().mockReturnValue(rpcUrls),
		getHostAddress: vi.fn().mockReturnValue(HOST),
		getBundlerUrl: vi.fn().mockReturnValue(undefined),
		getChainConfig: vi.fn().mockReturnValue({ chainId: 137 }),
		// The constructor kicks off initCache() without awaiting it.
		getConfiguredChainIds: vi.fn().mockReturnValue([]),
	} as unknown as FillerConfigService
	const cacheService = { getSolverSelection: () => null, setSolverSelection: () => {} } as unknown as CacheService
	const signer = {
		address: "0x1111111111111111111111111111111111111111",
	} as unknown as Signer

	return new ContractInteractionService(clientManager, configService, signer, cacheService)
}

describe("ContractInteractionService RPC selection", () => {
	beforeEach(() => {
		fromParams.mockClear()
		create.mockClear()
	})

	it("passes the operator's first endpoint when a chain has several RPCs", async () => {
		const service = makeService([PRIMARY, SECONDARY])

		await service.getIntentGateway("EVM-137", "EVM-137")

		// Regression: reading publicClient.transport.url yields undefined for a
		// fallback transport, and viem then silently substitutes its built-in
		// default (polygon.drpc.org), bypassing the configured endpoints.
		for (const call of fromParams.mock.calls) {
			expect((call[0] as { rpcUrl?: string }).rpcUrl).toBe(PRIMARY)
		}
		expect(fromParams).toHaveBeenCalledTimes(2)
	})

	it("passes the sole endpoint when a chain has exactly one RPC", async () => {
		const service = makeService([PRIMARY])

		await service.getIntentGateway("EVM-137", "EVM-137")

		for (const call of fromParams.mock.calls) {
			expect((call[0] as { rpcUrl?: string }).rpcUrl).toBe(PRIMARY)
		}
	})

	it("fails loudly rather than falling back to a public default when unconfigured", async () => {
		const service = makeService([])

		await expect(service.getIntentGateway("EVM-137", "EVM-137")).rejects.toThrow(/No RPC URL configured/)
	})
})
