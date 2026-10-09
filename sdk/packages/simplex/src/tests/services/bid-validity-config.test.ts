import { afterEach, beforeEach, describe, it, expect, vi } from "vitest"
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
	const HEAD_TIMESTAMP = 1_800_000_000n
	const SAMPLE_BLOCKS = 1_000n

	/**
	 * `nominalMs` is the block time the chain declares, in milliseconds as viem has it (Ethereum
	 * 12000, Base 2000, Arbitrum 250), or `undefined` for a chain that declares none.
	 * `actualSec` is what its recent blocks show and defaults to the declared figure; `null`
	 * makes the blocks unreadable. `chains` are the chain ids simplex is configured with, which it
	 * measures as it starts, and the first `failedReads` block reads fail.
	 *
	 * The window is the configured validity plus a 30s discovery allowance, converted once.
	 */
	function setup(
		nominalMs: number | undefined,
		bidValiditySeconds?: number,
		actualSec?: number | null,
		{ chains = [], failedReads = 0 }: { chains?: number[]; failedReads?: number } = {},
	) {
		const blockTimeSec = actualSec === undefined ? (nominalMs ?? 2000) / 1000 : actualSec
		let reads = 0
		const getBlock = vi.fn().mockImplementation(async (args: { blockTag?: string; blockNumber?: bigint }) => {
			if (blockTimeSec === null || ++reads <= failedReads) throw new Error("block not available")
			const number = args.blockNumber ?? HEAD
			return { number, timestamp: HEAD_TIMESTAMP - BigInt(Math.round(Number(HEAD - number) * blockTimeSec)) }
		})
		const clientManager = {
			getPublicClient: vi.fn().mockReturnValue({
				getBlockNumber: vi.fn().mockResolvedValue(HEAD),
				getBlock,
				chain: nominalMs === undefined ? {} : { blockTime: nominalMs },
			}),
		} as any
		const configService = {
			getBidValiditySeconds: () => bidValiditySeconds ?? 300,
			getConfiguredChainIds: () => chains,
			getHostAddress: () => "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3",
			getUsdcAsset: () => "0x",
			getUsdtAsset: () => "0x",
		} as any
		const svc = new ContractInteractionService(
			clientManager,
			configService,
			{ address: "0x1111111111111111111111111111111111111111" } as any,
			{ getTokenDecimals: () => undefined, setTokenDecimals: () => {} } as any,
		)
		const validUntil = (chain = "EVM-8453") => (svc as any).bidValidUntilBlock(chain) as Promise<bigint>
		return { svc, validUntil, getBlock }
	}

	// The startup warm-up also reads token decimals, which these tests leave out.
	beforeEach(() => {
		vi.spyOn(ContractInteractionService.prototype, "getFeeTokenWithDecimals").mockResolvedValue({} as any)
		vi.spyOn(ContractInteractionService.prototype, "getTokenDecimals").mockResolvedValue(18)
	})
	afterEach(() => {
		vi.restoreAllMocks()
	})

	const service = (nominalMs: number | undefined, bidValiditySeconds?: number, actualSec?: number | null) =>
		setup(nominalMs, bidValiditySeconds, actualSec).validUntil()

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

	it("uses the measured block time when the chain runs faster than it declares", async () => {
		// BSC declares 750ms and makes a block every 0.45s: 330s is 733.33 blocks, not 440.
		await expect(service(750, 300, 0.45)).resolves.toBe(HEAD + 734n)
	})

	it("measures a chain that declares no block time", async () => {
		// BSC Chapel at 0.45s and Polygon Amoy at 1s, neither with a declared figure.
		await expect(service(undefined, 300, 0.45)).resolves.toBe(HEAD + 734n)
		await expect(service(undefined, 300, 1)).resolves.toBe(HEAD + 330n)
	})

	it("keeps the declared block time when the sample ran slower", async () => {
		// A stall in the last 1,000 blocks must not shorten the bid once the chain recovers.
		await expect(service(2000, 300, 3)).resolves.toBe(HEAD + 165n)
	})

	it("samples the last 1,000 blocks", async () => {
		const { validUntil, getBlock } = setup(2000)
		await validUntil()

		expect(getBlock.mock.calls).toEqual([[{ blockTag: "latest" }], [{ blockNumber: HEAD - SAMPLE_BLOCKS }]])
	})

	it("measures every configured chain when simplex starts, and not again at a bid", async () => {
		const { svc, validUntil, getBlock } = setup(undefined, 300, 0.45, { chains: [97, 80002] })
		await svc.initCache()
		expect(getBlock).toHaveBeenCalledTimes(4)

		await expect(validUntil("EVM-97")).resolves.toBe(HEAD + 734n)
		await expect(validUntil("EVM-80002")).resolves.toBe(HEAD + 734n)

		expect(getBlock).toHaveBeenCalledTimes(4)
	})

	it("keeps the startup figure for as long as simplex runs", async () => {
		vi.useFakeTimers()
		try {
			const { svc, validUntil, getBlock } = setup(undefined, 300, 0.45, { chains: [97] })
			await svc.initCache()

			vi.advanceTimersByTime(7 * 24 * 60 * 60 * 1000)
			await expect(validUntil("EVM-97")).resolves.toBe(HEAD + 734n)

			expect(getBlock).toHaveBeenCalledTimes(2)
		} finally {
			vi.useRealTimers()
		}
	})

	it("measures a chain that was not there at startup once, at its first bid", async () => {
		const { validUntil, getBlock } = setup(undefined, 300, 0.45)
		expect(getBlock).not.toHaveBeenCalled()

		await expect(Promise.all([validUntil(), validUntil()])).resolves.toEqual([HEAD + 734n, HEAD + 734n])
		await validUntil()

		expect(getBlock).toHaveBeenCalledTimes(2)
	})

	it("measures each chain separately", async () => {
		const { validUntil, getBlock } = setup(undefined, 300, 0.45)

		await validUntil("EVM-97")
		await validUntil("EVM-80002")

		expect(getBlock).toHaveBeenCalledTimes(4)
	})

	it("falls back to the declared block time when the blocks cannot be read", async () => {
		await expect(service(750, 300, null)).resolves.toBe(HEAD + 440n)
	})

	it("falls back to a 2-second block time when there is neither a measurement nor a declared figure", async () => {
		await expect(service(undefined, 300, null)).resolves.toBe(HEAD + 165n)
	})

	it("measures at the first bid when the blocks could not be read at startup", async () => {
		const { svc, validUntil, getBlock } = setup(undefined, 300, 0.45, { chains: [97], failedReads: 1 })
		await svc.initCache()

		await expect(validUntil("EVM-97")).resolves.toBe(HEAD + 734n)
		await validUntil("EVM-97")

		// One failed read at startup, then the head and the earlier block once.
		expect(getBlock).toHaveBeenCalledTimes(3)
	})
})
