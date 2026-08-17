/**
 * Wire contracts between the UI server and the browser bundle. The server
 * types its response literals with these and `ui/src` imports them type-only,
 * so drift on either side is a compile error. Browser-safe: no runtime
 * dependencies beyond constants.
 */
import type { InitChainMeta, InitNetwork } from "@/cli/init/chains"
import type { VaultToml } from "@/config/filler-toml"
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
	sameAssetAskCurve: PriceCurvePoint[]
	testnetConfirmationPoints: CurvePoint[]
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
	/** Absent when the filler was started from a config object rather than a file. */
	configPath?: string
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

/** GET /api/strategies rows; PUT /api/strategies/:index(/curves) response */
export interface AdminStrategyDto {
	index: number
	/** Display label, e.g. "USDC/CNGN" (reference pairs carry a " (reference)" suffix). */
	exotic?: string
	token0: string
	token1: string
	pricingMode: "static" | "venue"
	sameToken: boolean
	referenceOnly: boolean
	/** Per-order cap in token0 units; absent for reference-only pairs (never consulted). */
	maxOrderSize?: string
	bid?: PriceCurvePoint[]
	ask?: PriceCurvePoint[]
}

/** GET /api/chains rows; one per `[[chains]]` entry in the running config. */
export interface ChainRowDto {
	chainId: number
	/** State machine id, e.g. "EVM-8453". */
	stateMachineId: string
	label: string
	rpcUrls: string[]
	bundlerUrl: string
	watchOnly: boolean
	/** False for rows added since boot — they only start filling after a restart. */
	running: boolean
}

/** GET /api/chains */
export interface ChainsDto {
	chains: ChainRowDto[]
	/** Every chain selectable on this network — the same catalog the setup wizard offers. */
	catalog: InitChainMeta[]
	/** Network the configured chains belong to; scopes the catalog and the Alchemy prefill. */
	network: InitNetwork
	/** True when `[simplex].watchOnly` is the global boolean — per-chain toggles are then frozen. */
	globalWatchOnly: boolean
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
}

/** GET /api/wallet/history rows: operator sends, vault sweeps/redeems and order fills. */
export interface WalletTxDto {
	/** Source-qualified ("wallet-3", "fill-17") — the two backing tables have overlapping ids. */
	id: string
	ts: number
	kind: "send" | "sweep" | "redeem" | "fill"
	chainId: number | null
	token: string | null
	amount: string | null
	to: string | null
	txHash: string
	sponsored: boolean | null
}

/** GET /api/config */
export interface ConfigDto {
	/** Absent when the filler was started from a config object rather than a file. */
	configPath?: string
	toml: string
	logLevel: string
	vaultConfigured: boolean
	allowlistUsers: string[]
	vaults: VaultToml[]
	sendTokens: Record<string, SendTokenOption[]>
	/** Registry vault catalog per running chain (state machine id), for selection UIs. */
	knownVaults: Record<string, KnownVault[]>
}
