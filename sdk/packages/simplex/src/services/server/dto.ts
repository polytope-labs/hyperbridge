/**
 * Wire contracts between the UI server and the browser bundle. The server
 * types its response literals with these and `ui/src` imports them type-only,
 * so drift on either side is a compile error. Browser-safe: no runtime
 * dependencies beyond constants.
 */
import type { InitChainMeta, InitNetwork } from "@/cli/init/chains"
import type { QueueConfig, VaultToml } from "@/config/filler-toml"
import type { CurvePoint, PriceCurvePoint } from "@/config/interpolated-curve"

export const LOG_LEVELS = ["trace", "debug", "info", "warn", "error"] as const

export interface KnownToken {
	symbol: string
	address: string
}

export interface KnownVault {
	label: string
	address: string
	asset: string
}

/** GET /api/setup/defaults */
export interface SetupDefaults {
	chains: InitChainMeta[]
	hyperbridgeWs: Record<InitNetwork, string>
	usdStables: string[]
	/** Every symbol the registry ships, chain-independent — custom assets must not shadow these. */
	registrySymbols: string[]
	sameAssetAskCurve: PriceCurvePoint[]
	testnetConfirmationPoints: CurvePoint[]
	queue: QueueConfig
	maxConcurrentOrders: number
	configPath: string
	/** Registry symbols resolvable per chain (state machine id), addresses included. */
	knownTokens: Record<string, KnownToken[]>
	knownVaults: Record<string, KnownVault[]>
}

/** GET /api/status in init mode */
export interface StatusInit {
	mode: "init"
	starting: boolean
	startError?: string
}

/** GET /api/status in operator mode */
export interface StatusOperator {
	mode: "operator"
	version: string
	uptimeSec: number
	paused: boolean
	halted: number[]
	watchOnly: Record<number, boolean>
	chains: number[]
	strategies: Array<{ index: number; exotic?: string }>
	strategyTypes: string[]
	configPath: string
	addresses?: { evm: string; substrate?: string }
	chainLabels?: Record<string, string>
}

export type Status = StatusInit | StatusOperator

/** GET /api/balances */
export interface BalanceSnapshot {
	updatedAt: number | null
	chains: Array<{
		chainId: number
		native?: { symbol: string; amount: number }
		usdc?: number
		usdt?: number
		exotics?: Array<{ symbol: string; amount: number }>
	}>
	hyperbridge?: { address: string; free: number; reserved: number }
}

/** GET /api/strategies rows; PUT /api/strategies/:index/curves response */
export interface AdminStrategyDto {
	index: number
	/** Display label, e.g. "USDC/CNGN" (reference pairs carry a " (reference)" suffix). */
	exotic?: string
	token0: string
	token1: string
	pricingMode: "static" | "venue"
	sameToken: boolean
	referenceOnly: boolean
	bid?: PriceCurvePoint[]
	ask?: PriceCurvePoint[]
}

/** GET /api/activity/orders rows; SSE /api/events frames */
export interface ActivityEventDto {
	id: number
	ts: number
	type: "detected" | "filled" | "executed" | "skipped" | "rebalance"
	orderId: string | null
	chainId: number | null
	strategy: string | null
	success: boolean | null
	reason: string | null
	volumeUsd: number | null
	profitUsd: number | null
	txHash: string | null
}

/** GET /api/activity/bids rows */
export interface BidDto {
	id: number
	commitment: string
	extrinsicHash: string | null
	success: boolean
	error: string | null
	createdAt: string
	retracted: boolean
}

export interface BidStatsDto {
	total: number
	successful: number
	failed: number
	retracted: number
	pendingRetraction: number
}

export interface SendTokenOption {
	symbol: string
	address: string
	vaultShare?: boolean
}

/** GET /api/config */
export interface ConfigDto {
	configPath: string
	toml: string
	logLevel: string
	vaultConfigured: boolean
	allowlistUsers: string[]
	vaults: VaultToml[]
	sendTokens: Record<string, SendTokenOption[]>
}
