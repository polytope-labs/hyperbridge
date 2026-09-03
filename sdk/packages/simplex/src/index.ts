/**
 * `@hyperbridge/simplex` — run a Hyperbridge intent filler inside your own process.
 *
 * ```ts
 * import { Simplex, privateKeySigner } from "@hyperbridge/simplex"
 * import { SqliteDataStore } from "@hyperbridge/simplex/sqlite"
 *
 * const simplex = await Simplex.start({
 *   config,
 *   signer: privateKeySigner(process.env.SOLVER_KEY as HexString),
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
 * separate entry point so the default install pulls in no native modules. Signing
 * is pluggable through {@link Signer}: the bundled backends implement it, and so
 * can a custody service of your own.
 */

// ─── Entry point ────────────────────────────────────────────────────────────

// The controllers are classes you receive from a Simplex, never construct. They
// are exported as values because that is what they are — exporting them as types
// made the emitted .d.ts declare runtime bindings that do not exist, so
// `import { PairController }` type-checked and was undefined at run time.
export {
	Simplex,
	PairController,
	ChainController,
	VaultController,
	AssetController,
	WalletController,
	RebalanceController,
} from "@/simplex"
export type {
	SimplexOptions,
	SimplexConfig,
	SimplexEvents,
	SimplexStatus,
	PairView,
	ChainInput,
	ChainView,
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
	// The binary's on-disk shape: a config plus the `[simplex.signer]` block.
	// Only needed if you load simplex TOML files yourself.
	FillerConfigFile,
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

// ─── Signing ────────────────────────────────────────────────────────────────
// `Signer` is the contract: an identity and three operations, with no viem types
// on it, so satisfying it never means matching this package's viem version. Each
// operation names what is being authorised, so a custody policy engine can read
// it. `viemSigner` adapts a viem account; `digestSigner` covers a backend that
// only signs 32-byte hashes.

export {
	privateKeySigner,
	turnkeySigner,
	mpcVaultSigner,
	viemSigner,
	digestSigner,
	createSigner,
	validateSignerConfig,
} from "@/services/wallet"
export type {
	Signer,
	Signature,
	TypedDataPayload,
	AuthorizationRequest,
	SignerTransaction,
	Eip7702Authorization,
} from "@/services/wallet/types"

// The `[simplex.signer]` TOML block the binary reads. It is not part of
// `SimplexConfig` — it lives on `FillerConfigFile`, and `createSigner` turns one
// into a `Signer`. Only useful if you load simplex config files yourself.
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

export { OrderScanner } from "@/scanner/order-scanner"
export { HyperbridgeScanner } from "@/scanner/hyperbridge-scanner"
export type {
	OrderScanner as OrderScannerContract,
	OrderScannerHandlers,
	OrderScannerOptions,
	HyperbridgeScanner as HyperbridgeScannerContract,
	HyperbridgeScannerHandlers,
	ScannerChainConfig,
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
