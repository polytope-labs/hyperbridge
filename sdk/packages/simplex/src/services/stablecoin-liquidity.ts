import { normalizeSymbol, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import type { AssetBalanceRow, BalanceSnapshot } from "@/services/BalanceProvider"

/**
 * Sums available USD-stable liquidity without estimating failed reads.
 * An empty asset set is a known zero; any unavailable stablecoin makes the
 * aggregate unknown because silently omitting it would understate liquidity.
 */
export function sumAvailableStablecoins(assets: readonly AssetBalanceRow[]): number | null {
	const stableAssets = assets.filter((asset) => USD_STABLE_SYMBOLS.has(normalizeSymbol(asset.symbol)))
	if (stableAssets.length === 0) return 0
	if (stableAssets.some((asset) => asset.available === null)) return null
	return stableAssets.reduce((total, asset) => total + (asset.available ?? 0), 0)
}

/** A cross-network total is trustworthy only when every configured chain refreshed. */
export function availableStablecoinLiquidity(snapshot: BalanceSnapshot | undefined): number | null {
	if (!snapshot || snapshot.status !== "fresh") return null
	return sumAvailableStablecoins(snapshot.chains.flatMap((chain) => chain.assets))
}
