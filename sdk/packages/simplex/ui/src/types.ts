// Hand-mirrored DTOs. Do not import from ../../src — those modules pull in
// node-only dependencies; the server re-validates everything anyway.

export type Network = "mainnet" | "testnet"

export interface CurvePoint {
	amount: string
	value: number
}

export interface PricePoint {
	amount: string
	price: string
}

export interface ChainDefault {
	chainId: number
	stateMachineId: string
	label: string
	network: Network
	alchemySubdomain?: string
	note?: string
}

export interface SetupDefaults {
	chains: ChainDefault[]
	hyperbridgeWs: Record<Network, string>
	stableBpsCurve: CurvePoint[]
	confirmationPolicies: Record<string, { points: CurvePoint[] }>
	testnetConfirmationPoints: CurvePoint[]
	queue: { maxRechecks: number; recheckDelayMs: number }
	maxConcurrentOrders: number
	configPath: string
}

export interface SignerConfig {
	type: "privateKey" | "mpcVault" | "turnkey"
	[key: string]: string
}

export interface StrategyConfig {
	type: "stable" | "hyperfx"
	bpsCurve?: CurvePoint[]
	confirmationPolicies?: Record<string, { points: CurvePoint[] }>
	maxOrderUsd?: number
	token1?: Record<string, string>
	bidPriceCurve?: PricePoint[]
	askPriceCurve?: PricePoint[]
	spreadBps?: number
}

export interface ChainEntry {
	rpcUrls: string[]
	bundlerUrl: string
}

export interface FillerConfig {
	simplex: {
		signer?: SignerConfig
		maxConcurrentOrders: number
		queue: { maxRechecks: number; recheckDelayMs: number }
		logging?: string
		watchOnly?: Record<string, boolean>
		substratePrivateKey: string
		hyperbridgeWsUrl: string
		gasFeeBump?: { maxPriorityFeePerGasBumpPercent?: number; maxFeePerGasBumpPercent?: number }
		overfillProtection?: { maxOverfillBps?: number; maxConsecutiveClamps?: number }
	}
	strategies: StrategyConfig[]
	chains: ChainEntry[]
	rebalancing?: {
		triggerPercentage: number
		baseBalances: { USDC?: Record<string, string>; USDT?: Record<string, string> }
	}
	binance?: { apiKey: string; apiSecret: string }
	vault?: {
		sweepIntervalMs?: number
		vaults: Array<{ chain: string; vault: string; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }>
	}
	allowlist?: { users?: string[] }
}

export interface StatusInit {
	mode: "init"
	starting: boolean
	startError?: string
}

export interface StatusOperator {
	mode: "operator"
	version: string
	uptimeSec: number
	paused: boolean
	halted: number[]
	watchOnly: Record<string, boolean>
	chains: number[]
	strategies: Array<{ index: number; exotic?: string }>
	strategyTypes: string[]
	configPath: string
}

export type Status = StatusInit | StatusOperator

export interface BalanceSnapshot {
	updatedAt: number | null
	chains: Array<{
		chainId: number
		native?: { symbol: string; amount: number }
		usdc?: number
		usdt?: number
		exotic?: { symbol: string; amount: number }
	}>
	hyperbridge?: { address: string; free: number; reserved: number }
}

export interface AdminStrategyDto {
	index: number
	exotic?: string
	pricingMode: "static" | "venue"
	bid?: PricePoint[]
	ask?: PricePoint[]
}
