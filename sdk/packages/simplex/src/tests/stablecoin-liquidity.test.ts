import { describe, expect, it } from "vitest"
import type { AssetBalanceRow, BalanceSnapshot } from "@/services/BalanceProvider"
import { availableStablecoinLiquidity, sumAvailableStablecoins } from "@/services/stablecoin-liquidity"

function asset(symbol: string, available: number | null): AssetBalanceRow {
	return {
		address: `0x${symbol}`,
		symbol,
		wallet: available,
		walletReserve: 0,
		vaultPosition: 0,
		vaultAvailable: 0,
		total: available,
		available,
		vaults: [],
		status: available === null ? "unavailable" : "fresh",
	}
}

function snapshot(status: BalanceSnapshot["status"], assets: AssetBalanceRow[]): BalanceSnapshot {
	return { updatedAt: Date.now(), status, chains: [{ chainId: 1, assets }], issues: [] }
}

describe("stablecoin liquidity", () => {
	it("includes every configured USD-stable symbol using normalized names", () => {
		expect(sumAvailableStablecoins([asset(" usdc ", 10), asset("USDT", 20), asset("dai", 30)])).toBe(60)
	})

	it("keeps a healthy network row usable during a partial refresh", () => {
		const assets = [asset("USDC", 10), asset("DAI", 5)]
		expect(sumAvailableStablecoins(assets)).toBe(15)
		expect(availableStablecoinLiquidity(snapshot("partial", assets))).toBeNull()
	})

	it("does not estimate around an unavailable stablecoin read", () => {
		expect(sumAvailableStablecoins([asset("USDC", 10), asset("DAI", null)])).toBeNull()
	})
})
