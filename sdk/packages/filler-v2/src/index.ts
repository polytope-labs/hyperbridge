export const version = "0.1.0"

// Core exports
export { IntentFiller } from "./core/filler"
export { EventMonitor } from "./core/event-monitor"

// Strategy exports
export { BasicFiller } from "./strategies/basic"

// Configuration exports
export { InterpolatedCurve, ConfirmationPolicy, FillerBpsPolicy } from "./config/interpolated-curve"
export type { CurvePoint, CurveConfig } from "./config/interpolated-curve"

// Service exports
export { ChainClientManager, ContractInteractionService } from "./services"
