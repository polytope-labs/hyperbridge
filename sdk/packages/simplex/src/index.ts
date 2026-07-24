export const version = "0.1.0"

// Core exports
export { IntentFiller } from "@/core/filler"
export { EventMonitor } from "@/core/event-monitor"

// Strategy exports
export { FXFiller } from "@/strategies/fx"

// Configuration exports
export { InterpolatedCurve, ConfirmationPolicy, FillerBpsPolicy } from "@/config/interpolated-curve"
export type { CurvePoint, CurveConfig } from "@/config/interpolated-curve"
export { PUBLIC_RPC_URLS, getPublicRpcUrls } from "@/config/public-rpcs"
export {
	AssetRegistry,
	KNOWN_ASSETS,
	USD_STABLE_SYMBOLS,
	validateAssetDefinitions,
	normalizeSymbol,
	isRegistrySymbol,
	registrySymbols,
} from "@/config/asset-registry"
export type { AssetDefinition, BuiltinAssetResolver } from "@/config/asset-registry"
export { validatePairConfigs } from "@/config/pairs"
export type { PairConfig } from "@/config/pairs"
export type { TradingPair } from "@/strategies/fx"

// Service exports
export { ChainClientManager, ContractInteractionService } from "@/services"

// Output funding
export type { OutputFundingConfig } from "@/funding/types"
export { UniswapV4LiquidityState, UniswapV4FundingPlanner } from "@/funding"
export { VaultLiquidityState, VaultFundingPlanner } from "@/funding"
