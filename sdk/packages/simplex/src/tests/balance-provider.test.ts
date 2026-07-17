import { describe, it, expect, vi } from "vitest"
import { BalanceProvider } from "@/services/BalanceProvider"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { deriveSubstrateKeyPair, generateSubstrateKey } from "@/services/substrate-key"

const USDC = "0xaaaa000000000000000000000000000000000001"
const USDT = "0xaaaa000000000000000000000000000000000002"
const EXOTIC = "0xaaaa000000000000000000000000000000000003"

function fakeClient() {
	return {
		getBalance: vi.fn().mockResolvedValue(2_000_000_000_000_000_000n), // 2 native
		readContract: vi.fn().mockImplementation(({ address, functionName }) => {
			if (functionName === "balanceOf") {
				if (address === USDC) return Promise.resolve(1_500_000_000n) // 1500 @ 6 decimals
				if (address === USDT) return Promise.resolve(2_500_000_000n) // 2500 @ 6 decimals
				if (address === EXOTIC) return Promise.resolve(10_000_000_000_000_000_000n) // 10 @ 18
			}
			if (functionName === "symbol") return Promise.resolve("cNGN")
			if (functionName === "decimals") return Promise.resolve(18)
			return Promise.reject(new Error(`unexpected call ${functionName}`))
		}),
	}
}

function makeProvider(client = fakeClient()) {
	const chainClientManager = { getPublicClient: vi.fn().mockReturnValue(client) } as unknown as ChainClientManager
	const configService = {
		getConfiguredChainIds: () => [8453],
		getUsdcAsset: () => USDC,
		getUsdcDecimals: () => 6,
		getUsdtAsset: () => USDT,
		getUsdtDecimals: () => 6,
	} as unknown as FillerConfigService
	return new BalanceProvider({
		chainClientManager,
		configService,
		fillerAddress: "0x1111111111111111111111111111111111111111",
		token1: { "EVM-8453": EXOTIC },
	})
}

describe("BalanceProvider", () => {
	it("collects native, stable and exotic balances into a snapshot", async () => {
		const provider = makeProvider()
		expect(provider.getSnapshot().updatedAt).toBeNull()

		const snapshot = await provider.refresh()
		expect(snapshot.updatedAt).toBeTypeOf("number")
		expect(snapshot.chains).toEqual([
			{
				chainId: 8453,
				native: { symbol: "ETH", amount: 2 },
				usdc: 1500,
				usdt: 2500,
				exotic: { symbol: "cNGN", amount: 10 },
			},
		])
		expect(provider.getSnapshot()).toBe(snapshot)
	})

	it("keeps partial results when individual reads fail", async () => {
		const client = fakeClient()
		client.getBalance.mockRejectedValue(new Error("rpc down"))
		const provider = makeProvider(client)

		const snapshot = await provider.refresh()
		expect(snapshot.chains[0].native).toBeUndefined()
		expect(snapshot.chains[0].usdc).toBe(1500)
	})

	it("start()/stop() own the refresh timers", async () => {
		vi.useFakeTimers()
		try {
			const provider = makeProvider()
			const refreshSpy = vi.spyOn(provider, "refresh").mockResolvedValue(provider.getSnapshot())

			provider.start()
			await vi.advanceTimersByTimeAsync(5_000)
			expect(refreshSpy).toHaveBeenCalledTimes(1)
			await vi.advanceTimersByTimeAsync(60_000)
			expect(refreshSpy).toHaveBeenCalledTimes(2)

			provider.stop()
			await vi.advanceTimersByTimeAsync(180_000)
			expect(refreshSpy).toHaveBeenCalledTimes(2)
		} finally {
			vi.useRealTimers()
		}
	})
})

describe("substrate-key", () => {
	it("derives the same address for 0x-prefixed and bare hex seeds", async () => {
		const seed = "1234567890123456789012345678901234567890123456789012345678901234"
		const bare = await deriveSubstrateKeyPair(seed)
		const prefixed = await deriveSubstrateKeyPair(`0x${seed}`)
		expect(bare.address).toBe(prefixed.address)
	})

	it("derives from mnemonics and URIs", async () => {
		const mnemonic = "bottom drive obey lake curtain smoke basket hold race lonely fit walk"
		const fromMnemonic = await deriveSubstrateKeyPair(mnemonic)
		const fromUri = await deriveSubstrateKeyPair("//Alice")
		expect(fromMnemonic.address).toMatch(/^5/)
		expect(fromUri.address).toMatch(/^5/)
		expect(fromMnemonic.address).not.toBe(fromUri.address)
	})

	it("generates a mnemonic whose derivation matches the returned address", async () => {
		const { mnemonic, address } = await generateSubstrateKey()
		expect(mnemonic.split(" ")).toHaveLength(12)
		const pair = await deriveSubstrateKeyPair(mnemonic)
		expect(pair.address).toBe(address)
	})
})
