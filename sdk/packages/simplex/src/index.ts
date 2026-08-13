/**
 * `@hyperbridge/simplex` — run a Hyperbridge intent filler inside your own process.
 *
 * ```ts
 * import { Simplex } from "@hyperbridge/simplex"
 * import { SqliteDataStore } from "@hyperbridge/simplex/sqlite"
 *
 * const simplex = await Simplex.start({
 *   config,
 *   data: new SqliteDataStore("./simplex-data"),
 * })
 *
 * simplex.on("order:filled", ({ orderId, profitUsd }) => {
 *   console.log(`filled ${orderId} for $${profitUsd}`)
 * })
 *
 * await simplex.pairs.setCurve(0, "ask", [{ amount: "0", price: "1550" }])
 * await simplex.stop()
 * ```
 *
 * The `Simplex` instance is the whole API: it starts the filler, exposes every
 * runtime control as a namespaced controller, and emits typed events. Persistence
 * is pluggable through {@link SimplexDataStore} — the SQLite implementation is a
 * separate entry point so the default install pulls in no native modules.
 */

// ─── Entry point ────────────────────────────────────────────────────────────

export { Simplex } from "@/simplex"
export type {
	SimplexOptions,
	SimplexConfig,
	SimplexEvents,
	SimplexStatus,
	PairController,
	PairView,
	ChainController,
	ChainInput,
	ChainView,
	VaultController,
	AssetController,
	WalletController,
	RebalanceController,
} from "@/simplex"

// ─── Persistence ────────────────────────────────────────────────────────────
// The SQLite implementation lives at `@hyperbridge/simplex/sqlite`; it needs the
// optional `better-sqlite3` native module, which nothing here does.

export { MemoryDataStore } from "@/data/memory"
export type {
	SimplexDataStore,
	BidStore,
	ActivityStore,
	StateStore,
	StoredBid,
	BidInsert,
	BidStats,
	ActivityEvent,
	ActivityInsert,
	ActivityType,
	WalletTx,
	WalletTxKind,
	RuntimeState,
} from "@/data/types"

// ─── Config ─────────────────────────────────────────────────────────────────
// `SimplexConfig` is a plain object — no TOML required. These validators are
// pure and run the same rules boot does, so a config that passes here starts.

export { validateConfig, assertConfirmationCoverage, validateVaultToml, validateUniswapV4Positions } from "@/config/filler-toml"
export type {
	FillerTomlConfig,
	ChainConfirmationPolicy,
	QueueConfig,
	RebalancingConfig,
	BinanceConfig,
	VaultToml,
	VaultTomlConfig,
	UniswapV4PositionToml,
} from "@/config/filler-toml"

export { validatePairConfigs, unanchoredToken0Symbols, pickAnchorStable } from "@/config/pairs"
export type { PairConfig } from "@/config/pairs"

export {
	validateAssetDefinitions,
	normalizeSymbol,
	isRegistrySymbol,
	registrySymbols,
	USD_STABLE_SYMBOLS,
} from "@/config/asset-registry"
export type { AssetDefinition } from "@/config/asset-registry"

export { bookCrossedAt, parseChainKey, formatChainKey } from "@/config/interpolated-curve"
export type { PriceCurvePoint, PriceCurveConfig, CurvePoint, CurveConfig } from "@/config/interpolated-curve"

export { SignerType } from "@/services/wallet/types"
export type {
	SignerConfig,
	PrivateKeySignerConfig,
	MpcVaultSignerConfig,
	TurnkeySignerConfig,
} from "@/services/wallet/types"

export type { AllowlistConfig, UserProvidedChainConfig, ResolvedChainConfig } from "@/services/FillerConfigService"
export type { BalanceSnapshot, ChainBalanceRow, HyperbridgeBalance } from "@/services/BalanceProvider"

// ─── Shared scanners ────────────────────────────────────────────────────────
// Scanning a chain is identical work for every filler, so the default sources
// share one loop per (chain, gateway, endpoints) and one Hyperbridge poll per
// endpoint across every Simplex in the process. Implement these contracts to
// feed fillers from somewhere else — another process, an indexer, a bus.

export { SharedOrderSource, SharedHyperbridgeSource, sharedScanners } from "@/scanner/registry"
export { scanKey } from "@/scanner/types"
export type {
	OrderSource,
	OrderSourceHandlers,
	HyperbridgeSource,
	HyperbridgeHandlers,
	ScanTarget,
	ScannedOrder,
	ScannedFill,
	Subscription,
} from "@/scanner/types"

// ─── Logging ────────────────────────────────────────────────────────────────
// Silent until a sink is registered. Each filler owns a LoggerContext, so
// `SimplexOptions.logger` and `simplex.setLogLevel` are scoped to that filler.
// The `addLogSink`/`configureLogger`/`getLogger` trio drives the process-wide
// fallback context, which only code outside a filler's lifetime writes to.

export { addLogSink, configureLogger, getLogger, LoggerContext } from "@/services/Logger"
export type { LogLevel, Logger, LogSink } from "@/services/Logger"
