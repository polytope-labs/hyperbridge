import { describe, it, expect, vi } from "vitest"
import { BalanceProvider, type VaultBalanceSource } from "@/services/BalanceProvider"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { deriveSubstrateKeyPair, POLKADOT_UNIFIED_SS58_PREFIX } from "@/services/substrate-key"

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

function makeProvider(
	client = fakeClient(),
	vaultBalances?: VaultBalanceSource,
	configOverrides: Partial<FillerConfigService> = {},
) {
	const chainClientManager = { getPublicClient: vi.fn().mockReturnValue(client) } as unknown as ChainClientManager
	const configService = {
		getConfiguredChainIds: () => [8453],
		getUsdcAsset: () => USDC,
		getUsdcDecimals: () => 6,
		getUsdtAsset: () => USDT,
		getUsdtDecimals: () => 6,
		...configOverrides,
	} as unknown as FillerConfigService
	return new BalanceProvider({
		chainClientManager,
		configService,
		fillerAddress: "0x1111111111111111111111111111111111111111",
		token1: { "EVM-8453": [EXOTIC] },
		vaultBalances,
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
				exotics: [{ symbol: "cNGN", amount: 10 }],
				assets: [
					{
						address: USDC,
						symbol: "USDC",
						wallet: 1500,
						walletReserve: 0,
						vaultPosition: 0,
						vaultAvailable: 0,
						total: 1500,
						available: 1500,
						vaults: [],
						status: "fresh",
					},
					{
						address: USDT,
						symbol: "USDT",
						wallet: 2500,
						walletReserve: 0,
						vaultPosition: 0,
						vaultAvailable: 0,
						total: 2500,
						available: 2500,
						vaults: [],
						status: "fresh",
					},
					{
						address: EXOTIC,
						symbol: "cNGN",
						wallet: 10,
						walletReserve: 0,
						vaultPosition: 0,
						vaultAvailable: 0,
						total: 10,
						available: 10,
						vaults: [],
						status: "fresh",
					},
				],
			},
		])
		expect(snapshot.status).toBe("fresh")
		expect(snapshot.issues).toEqual([])
		expect(provider.getSnapshot()).toBe(snapshot)
	})

	it("keeps partial results when individual reads fail", async () => {
		const client = fakeClient()
		client.getBalance.mockRejectedValue(new Error("rpc down"))
		const provider = makeProvider(client)

		const snapshot = await provider.refresh()
		expect(snapshot.chains[0].native).toBeUndefined()
		expect(snapshot.chains[0].usdc).toBe(1500)
		expect(snapshot.status).toBe("partial")
		expect(snapshot.issues).toEqual([
			{ chainId: 8453, source: "native", message: "rpc down" },
		])
	})

	it("combines wallet and vault balances without hiding reserved or unavailable liquidity", async () => {
		const vaultBalances: VaultBalanceSource = {
			getBalanceSnapshot: vi.fn().mockResolvedValue([
				{
					chain: "EVM-8453",
					vault: "0xbbbb000000000000000000000000000000000001",
					asset: USDC,
					symbol: "USDC",
					decimals: 6,
					positionAssets: 3_000_000_000n,
					availableAssets: 2_500_000_000n,
					walletReserve: 100_000_000n,
				},
			]),
		}
		const snapshot = await makeProvider(fakeClient(), vaultBalances).refresh()
		const usdc = snapshot.chains[0].assets.find((asset) => asset.symbol === "USDC")

		expect(usdc).toMatchObject({
			wallet: 1500,
			walletReserve: 100,
			vaultPosition: 3000,
			vaultAvailable: 2500,
			total: 4500,
			available: 3900,
			status: "fresh",
		})
	})

	it("marks aggregate values unavailable when the vault source fails", async () => {
		const vaultBalances: VaultBalanceSource = {
			getBalanceSnapshot: vi.fn().mockRejectedValue(new Error("vault rpc down")),
		}
		const snapshot = await makeProvider(fakeClient(), vaultBalances).refresh()
		const usdc = snapshot.chains[0].assets.find((asset) => asset.symbol === "USDC")

		expect(snapshot.status).toBe("partial")
		expect(snapshot.issues).toContainEqual({ chainId: 8453, source: "vault", message: "vault rpc down" })
		expect(usdc).toMatchObject({ wallet: 1500, total: null, available: null, status: "partial" })
	})

	it("isolates a vault refresh failure to the affected chain", async () => {
		const vaultBalances: VaultBalanceSource = {
			getBalanceSnapshot: vi.fn().mockImplementation((chain) => {
				if (chain === "EVM-8453") return Promise.reject(new Error("base vault unavailable"))
				return Promise.resolve([
					{
						chain: "EVM-42161",
						vault: "0xbbbb000000000000000000000000000000000001",
						asset: USDC,
						symbol: "USDC",
						decimals: 6,
						positionAssets: 3_000_000_000n,
						availableAssets: 2_500_000_000n,
						walletReserve: 100_000_000n,
					},
				])
			}),
		}
		const snapshot = await makeProvider(fakeClient(), vaultBalances, {
			getConfiguredChainIds: (() => [8453, 42161]) as FillerConfigService["getConfiguredChainIds"],
		}).refresh()

		expect(snapshot.chains[0].assets.find((asset) => asset.symbol === "USDC")?.total).toBeNull()
		expect(snapshot.chains[1].assets.find((asset) => asset.symbol === "USDC")).toMatchObject({
			total: 4500,
			available: 3900,
			status: "fresh",
		})
		expect(snapshot.issues).toEqual([
			{ chainId: 8453, source: "vault", message: "base vault unavailable" },
		])
	})

	it("does not guess token units when an exotic token's decimals cannot be read", async () => {
		const client = fakeClient()
		const originalRead = client.readContract.getMockImplementation()!
		client.readContract.mockImplementation((args) => {
			if (args.address === EXOTIC && args.functionName === "decimals") {
				return Promise.reject(new Error("decimals unavailable"))
			}
			return originalRead(args)
		})

		const snapshot = await makeProvider(client).refresh()
		const exotic = snapshot.chains[0].assets.find((asset) => asset.address === EXOTIC)

		expect(exotic).toMatchObject({ wallet: null, total: null, available: null, status: "unavailable" })
		expect(snapshot.issues).toContainEqual({
			chainId: 8453,
			source: "token",
			asset: EXOTIC,
			message: "Decimals read failed: decimals unavailable",
		})
	})

	it("skips an unsupported zero-address stable without reporting a false RPC failure", async () => {
		const snapshot = await makeProvider(fakeClient(), undefined, {
			getUsdtAsset: (() => "0x") as FillerConfigService["getUsdtAsset"],
		}).refresh()

		expect(snapshot.chains[0].assets.some((asset) => asset.symbol === "USDT")).toBe(false)
		expect(snapshot.issues.some((issue) => issue.asset === "USDT")).toBe(false)
	})

	it("start()/stop() own the refresh timers", async () => {
		vi.useFakeTimers()
		try {
			const provider = makeProvider()
			const refreshSpy = vi.spyOn(provider, "refresh").mockResolvedValue(provider.getSnapshot())

			await provider.start()
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
	// 20s: first touch of @polkadot/wasm-crypto, which is slow under suite load.
	it("derives the same address for 0x-prefixed and bare hex seeds", async () => {
		const seed = "1234567890123456789012345678901234567890123456789012345678901234"
		const bare = await deriveSubstrateKeyPair(seed)
		const prefixed = await deriveSubstrateKeyPair(`0x${seed}`)
		expect(bare.address).toBe(prefixed.address)
	}, 20_000)

	it("derives from mnemonics and URIs", async () => {
		const mnemonic = "bottom drive obey lake curtain smoke basket hold race lonely fit walk"
		const fromMnemonic = await deriveSubstrateKeyPair(mnemonic)
		const fromUri = await deriveSubstrateKeyPair("//Alice")
		const { encodeAddress } = await import("@polkadot/util-crypto")
		expect(fromMnemonic.address).toBe(encodeAddress(fromMnemonic.publicKey, POLKADOT_UNIFIED_SS58_PREFIX))
		expect(fromUri.address).toBe(encodeAddress(fromUri.publicKey, POLKADOT_UNIFIED_SS58_PREFIX))
		expect(fromMnemonic.address).not.toBe(fromUri.address)
	})
})
