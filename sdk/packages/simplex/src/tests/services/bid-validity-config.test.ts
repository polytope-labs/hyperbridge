import { describe, it, expect, vi } from "vitest"
import { FillerConfigService } from "@/services/FillerConfigService"
import { ContractInteractionService } from "@/services/ContractInteractionService"

/**
 * `bidValiditySeconds` bounds how long a signed bid stays executable. It is the only thing
 * bounding it: the order's own deadline is placer-chosen with no ceiling, and retracting a bid
 * on Hyperbridge does not reach the destination chain. So the default matters as much as the
 * override — a filler that never sets it must still get a bounded quote.
 */

const CHAINS = [{ chainId: 8453, rpcUrls: ["https://rpc.example"] }] as any

function service(fillerConfig?: Record<string, unknown>) {
	return new FillerConfigService(CHAINS, fillerConfig as any)
}

describe("bidValiditySeconds", () => {
	it("defaults to 5 minutes when the operator does not set it", () => {
		expect(service().getBidValiditySeconds()).toBe(300)
	})

	it("takes the configured value when set", () => {
		expect(service({ bidValiditySeconds: 900 }).getBidValiditySeconds()).toBe(900)
	})

	it("is honoured at zero rather than falling back to the default", () => {
		// `?? 300` and `|| 300` differ here, and only the first is right: 0 is a deliberate
		// "no bound", not an absent value.
		expect(service({ bidValiditySeconds: 0 }).getBidValiditySeconds()).toBe(0)
	})
})

describe("bidValidUntilBlock", () => {
	const HEAD = 1_000_000n

	/**
	 * `blockTime` is milliseconds in viem (Ethereum 12000, Base 2000, Arbitrum 250), so the
	 * helper divides by 1000. Passing `undefined` exercises the seconds-denominated fallback.
	 */
	function service(blockTimeMs: number | undefined, bidValiditySeconds?: number) {
		const clientManager = {
			getPublicClient: vi.fn().mockReturnValue({
				getBlockNumber: vi.fn().mockResolvedValue(HEAD),
				chain: blockTimeMs === undefined ? {} : { blockTime: blockTimeMs },
			}),
		} as any
		const configService = {
			getBidValiditySeconds: () => bidValiditySeconds ?? 300,
			getConfiguredChainIds: () => [],
			getHostAddress: () => "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3",
		} as any
		const svc = new ContractInteractionService(
			clientManager,
			configService,
			{ address: "0x1111111111111111111111111111111111111111" } as any,
			{ getTokenDecimals: () => undefined, setTokenDecimals: () => {} } as any,
		)
		return (svc as any).bidValidUntilBlock("EVM-8453") as Promise<bigint>
	}

	it("converts seconds to blocks at the chain's block time, plus the pad", async () => {
		// Base: 2000ms blocks => 300s is 150 blocks, +5 pad.
		await expect(service(2000)).resolves.toBe(HEAD + 150n + 5n)
	})

	it("scales with a slower chain", async () => {
		// Ethereum: 12000ms blocks => 300s is 25 blocks, +5 pad.
		await expect(service(12000)).resolves.toBe(HEAD + 25n + 5n)
	})

	it("scales with a sub-second chain", async () => {
		// Arbitrum: 250ms blocks => 300s is 1200 blocks, +5 pad.
		await expect(service(250)).resolves.toBe(HEAD + 1200n + 5n)
	})

	it("rounds the block count up rather than down", async () => {
		// 750ms blocks => 100s is 133.33 blocks; short would drop winnable bids.
		await expect(service(750, 100)).resolves.toBe(HEAD + 134n + 5n)
	})

	it("falls back to a 2-second block time when the chain does not declare one", async () => {
		await expect(service(undefined)).resolves.toBe(HEAD + 150n + 5n)
	})
})
