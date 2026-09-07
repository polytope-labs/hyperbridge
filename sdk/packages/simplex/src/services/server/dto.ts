/**
 * Wire contracts between the UI server and the browser bundle. The server
 * types its response literals with these and `ui/src` imports them type-only,
 * so drift on either side is a compile error. Browser-safe: no runtime
 * dependencies beyond constants.
 */
import type { InitChainMeta, InitNetwork } from "@/cli/init/chains"
import type { VaultToml } from "@/config/filler-toml"
import type { CurvePoint, PriceCurvePoint } from "@/config/interpolated-curve"
import type { BalanceSnapshot as RuntimeBalanceSnapshot } from "@/services/BalanceProvider"
import type { VaultSweepSkipReason } from "@/funding/vault/VaultFundingPlanner"
import type { ActivityType, OrderSummary } from "@/data/types"

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

/** GET /api/balances — shared with the runtime collector to prevent contract drift. */
export type BalanceSnapshot = RuntimeBalanceSnapshot

/**
 * POST /api/vault/sweep response. `submitted` is empty when the pass found nothing it could
 * deposit; `skipped` says why per vault, so the dashboard can tell a wallet below its threshold
 * from a vault that is refusing deposits. Amounts are in the underlying token's display units.
 */
export interface VaultSweepDto {
	ok: true
	submitted: {
		chain: string
		txHash: string
		sponsored: boolean
		deposits: { vault: string; symbol: string; amount: string }[]
	}[]
	skipped: {
		chain: string
		vault: string
		symbol: string
		reason: VaultSweepSkipReason
		walletBalance?: string
		threshold?: string
	}[]
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
	type: ActivityType
	orderId: string | null
	chainId: number | null
	strategy: string | null
	success: boolean | null
	reason: string | null
	volumeUsd: number | null
	profitUsd: number | null
	txHash: string | null
	order: OrderSummary | null
}

export type { ActivityType, OrderLeg, OrderSummary } from "@/data/types"

/** GET /api/activity/history — one page of orders, each with its rows and Hyperbridge bids. */
export interface OrderHistoryDto {
	page: number
	pageSize: number
	total: number
	/** Network the running chains belong to; picks the Hyperbridge explorer for bid extrinsics. */
	network: InitNetwork
	orders: Array<{
		orderId: string
		/** Newest first. */
		events: ActivityEventDto[]
		/** Bids submitted for this order's commitment, newest first. */
		bids: BidDto[]
	}>
	/** Newest events with no order (rebalances), for the footer of the first page. */
	other: ActivityEventDto[]
}

/** GET /api/activity/bids rows */
export interface BidDto {
	id: number
	commitment: string
	extrinsicHash: string | null
	success: boolean
	error: string | null
	/** SQLite-style "YYYY-MM-DD HH:MM:SS" in UTC. */
	createdAt: string
	retracted: boolean
	retractedAt: string | null
	retractExtrinsicHash: string | null
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
	/** Curated vault name when `to` is a known vault (sweep/redeem), else null. */
	label: string | null
	/** What came into the wallet (fill: the order's input; sweep: vault shares; redeem: the underlying). */
	in: LedgerLeg | null
	/** What left the wallet (fill: the order's output; sweep: the underlying; redeem: shares; send: the token). */
	out: LedgerLeg | null
}

/** One side of a ledger row. `decimals` null means `amount` is already a decimal string. */
export interface LedgerLeg {
	symbol: string
	amount: string
	decimals: number | null
	/** Symbol whose logo to show; for vault shares this is the underlying (stataUSDC → USDC). */
	icon: string
	/** True for vault share tokens, which render with a vault badge over the underlying's logo. */
	vault: boolean
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
	/**
	 * Registry vault catalog per chain (state machine id) for every chain on the
	 * running network, not only the running ones; the editor disables rows for
	 * chains that are not enabled. Running chains are always present, possibly empty.
	 */
	knownVaults: Record<string, KnownVault[]>
	/** Remote-access summary for the Operations list; absent when the filler has no tunnel. */
	tunnel?: { enabled: boolean; devices: number }
}

/** Where the remote-access tunnel is in its lifecycle. */
export type TunnelState = "disabled" | "connecting" | "connected" | "reconnecting" | "disconnected" | "error"

export interface TunnelDeviceDto {
	/** `SHA256:…` of the device's public key. */
	fingerprint: string
	label: string
	/** Unix milliseconds; 0 for a line added to `authorized_keys` by hand. */
	addedAt: number
}

/** GET /api/tunnel */
export interface TunnelStatusDto {
	enabled: boolean
	state: TunnelState
	/** `host:port` of the relay in use. */
	relay: string
	/** Pinned relay host key, once known. */
	relayFingerprint?: string
	/** Public port the relay leased to this simplex; what the phone connects to. */
	port?: number
	connectedAt?: number
	lastError?: string
	/** The embedded SSH server's host key, which the phone must pin. */
	hostFingerprint: string
	/** This simplex's identity toward the relay. */
	operatorFingerprint: string
	devices: TunnelDeviceDto[]
	/** Device sessions open right now. */
	activeConnections: number
	/**
	 * The connection an operator types into their SSH app. Overlaps `port` and
	 * `hostFingerprint` on purpose: this is the block the dashboard renders, so
	 * it stays one shape whether it comes from here or from pairing.
	 */
	connection: TunnelConnectionDto
}

/** Everything a phone's SSH app needs to reach this dashboard. */
export interface TunnelConnectionDto {
	host: string
	/** Absent until the relay has leased a port. */
	port?: number
	username: string
	/** The embedded SSH server's host key, which the phone pins. */
	hostFingerprint: string
	/** `-L` argument: local port to the UI bind. */
	localForward: string
}

/**
 * POST /api/tunnel/devices. When the request carried the phone's own public
 * key there is no private key here; when simplex generated the pair, the
 * private key is returned once and never stored.
 */
export interface TunnelNewDeviceDto {
	device: TunnelDeviceDto
	/** OpenSSH-format private key for the phone; absent for a pasted public key. */
	privateKey?: string
	publicKey: string
	connection: TunnelConnectionDto
}
