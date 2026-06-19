export const version = "0.1.0"

// Core exports
export { IntentFiller } from "@/core/filler"
export { EventMonitor } from "@/core/event-monitor"

// Strategy exports
export { StableFiller } from "@/strategies/stable"
export { FXFiller, AccumulationSide } from "@/strategies/fx"

// Configuration exports
export { InterpolatedCurve, ConfirmationPolicy, FillerBpsPolicy } from "@/config/interpolated-curve"
export type { CurvePoint, CurveConfig } from "@/config/interpolated-curve"

// Service exports
export { ChainClientManager, ContractInteractionService } from "@/services"

// Output funding
export type { OutputFundingConfig } from "@/funding/types"
export { UniswapV4LiquidityState, UniswapV4FundingPlanner } from "@/funding"
export { VaultLiquidityState, VaultFundingPlanner } from "@/funding"
