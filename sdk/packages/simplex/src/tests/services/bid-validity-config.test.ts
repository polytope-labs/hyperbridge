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
	 *
	 * The window is the configured validity plus a 30s discovery allowance, converted once.
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

	it("converts the validity window plus the discovery pad at the chain's block time", async () => {
		// Base: 2000ms blocks => (300 + 30)s is 165 blocks.
		await expect(service(2000)).resolves.toBe(HEAD + 165n)
	})

	it("scales with a slower chain", async () => {
		// Ethereum: 12000ms blocks => 330s is 27.5 blocks, rounded up to 28.
		await expect(service(12000)).resolves.toBe(HEAD + 28n)
	})

	it("scales with a sub-second chain", async () => {
		// Arbitrum: 250ms blocks => 330s is 1320 blocks.
		await expect(service(250)).resolves.toBe(HEAD + 1320n)
	})

	it("gives the discovery pad the same wall-clock weight on every chain", async () => {
		// The point of denominating it in seconds: 30s is 15 blocks on Base and 120 on
		// Arbitrum, not one flat count worth wildly different amounts of time.
		const [base, arbitrum] = await Promise.all([service(2000, 0), service(250, 0)])
		expect(base - HEAD).toBe(15n)
		expect(arbitrum - HEAD).toBe(120n)
	})

	it("rounds the block count up rather than down", async () => {
		// 750ms blocks => (100 + 30)s is 173.33 blocks; short would drop winnable bids.
		await expect(service(750, 100)).resolves.toBe(HEAD + 174n)
	})

	it("falls back to a 2-second block time when the chain does not declare one", async () => {
		await expect(service(undefined)).resolves.toBe(HEAD + 165n)
	})
})
