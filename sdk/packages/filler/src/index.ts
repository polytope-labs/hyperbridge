export const version = "0.1.0"

// Core exports
export { IntentFiller } from "./core/filler"
export { EventMonitor } from "./core/event-monitor"

// Strategy exports
export { BasicFiller } from "./strategies/basic"
export { StableSwapFiller } from "./strategies/swap"

// Configuration exports
export { ConfirmationPolicy } from "./config/confirmation-policy"

// Service exports
export { ChainClientManager, ContractInteractionService } from "./services"
