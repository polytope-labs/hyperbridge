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

export interface KnownToken {
	symbol: string
	address: string
	decimals?: number
}

export interface KnownVault {
	label: string
	address: string
	asset: string
}

export interface SetupDefaults {
	chains: ChainDefault[]
	hyperbridgeWs: Record<Network, string>
	usdStables: string[]
	/** Every symbol the registry ships, chain-independent — custom assets must not shadow these. */
	registrySymbols: string[]
	sameAssetAskCurve: PricePoint[]
	confirmationPolicies: Record<string, { points: CurvePoint[] }>
	testnetConfirmationPoints: CurvePoint[]
	queue: { maxRechecks: number; recheckDelayMs: number }
	maxConcurrentOrders: number
	configPath: string
	/** Registry symbols resolvable per chain (state machine id), addresses included. */
	knownTokens: Record<string, KnownToken[]>
	knownVaults: Record<string, KnownVault[]>
}

export interface SignerConfig {
	type: "privateKey" | "mpcVault" | "turnkey"
	[key: string]: string | undefined
}

export interface PairConfig {
	token0: string
	token1: string
	maxOrderSize?: string
	referenceOnly?: boolean
	bidPriceCurve?: PricePoint[]
	askPriceCurve?: PricePoint[]
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
	assets?: Record<string, Record<string, string>>
	pairs?: PairConfig[]
	confirmationPolicies?: Record<string, { points: CurvePoint[] }>
	chains: ChainEntry[]
	rebalancing?: {
		triggerPercentage: number
		baseBalances: { USDC?: Record<string, string>; USDT?: Record<string, string> }
	}
	binance?: { apiKey: string; apiSecret: string }
	vault?: {
		sweepIntervalMs?: number
		vaults?: Array<{ chain: string; vault: string; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }>
		uniswapV4?: {
			positions?: Array<{ chain: string; tokenId: string; referencePrice?: string; maxDeviationBps?: number }>
			side?: "bid" | "ask"
			spreadBps?: number
		}
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
	addresses?: { evm: string; substrate?: string }
	chainLabels?: Record<string, string>
}

export type Status = StatusInit | StatusOperator

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

export interface AdminStrategyDto {
	index: number
	exotic?: string
	pricingMode: "static" | "venue"
	sameToken?: boolean
	referenceOnly?: boolean
	bid?: PricePoint[]
	ask?: PricePoint[]
}

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

export interface ConfigDto {
	configPath: string
	toml: string
	logLevel: string
	vaultConfigured: boolean
	allowlistUsers: string[]
	vaults: Array<{ chain: string; vault: string; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }>
	sendTokens: Record<string, SendTokenOption[]>
}
