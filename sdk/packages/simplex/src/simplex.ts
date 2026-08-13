import { EventEmitter } from "node:events"
import { Decimal } from "decimal.js"
import type { HexString, Order } from "@hyperbridge/sdk"
import { adminStrategyFor, bootFiller, tradingPairFrom, type FillerRuntime } from "@/core/boot"
import { FillerPricePolicy, formatChainKey, type PriceCurvePoint } from "@/config/interpolated-curve"
import { normalizeSymbol, type AssetDefinition } from "@/config/asset-registry"
import { assertPairSymbolsResolve, validatePairConfigs, type PairConfig } from "@/config/pairs"
import { assertConfirmationCoverage, type VaultToml } from "@/config/filler-toml"
import type { ChainConfirmationPolicy, FillerTomlConfig, RebalancingConfig } from "@/config/filler-toml"
import { resolveChainConfigs, validateRpcUrls, type AllowlistConfig } from "@/services/FillerConfigService"
import { LoggerContext, type Logger, type LogLevel, type LogSink } from "@/services/Logger"
import type { ActivityEvent, BidStats, SimplexDataStore, StoredBid, WalletTx } from "@/data/types"
import { MemoryDataStore } from "@/data/memory"
import { OrderStream } from "@/scanner/order-stream"
import { HyperbridgeStream } from "@/scanner/hyperbridge-stream"
import type { BalanceSnapshot } from "@/services/BalanceProvider"

/** Configuration for a filler. The same shape the TOML file parses into. */
export type SimplexConfig = FillerTomlConfig

export interface SimplexOptions {
	config: SimplexConfig
	/**
	 * Persistence backend. Defaults to an in-process store that is lost on exit —
	 * fine for tests and watch-only observers, but a filler that submits bids
	 * should pass a durable one (`SqliteDataStore` from
	 * `@hyperbridge/simplex/sqlite`, or your own). A lost bid record is a
	 * Hyperbridge deposit nobody retracts.
	 */
	data?: SimplexDataStore
	/**
	 * Where this config was loaded from, if anywhere. Only used to write runtime
	 * edits back to disk; omit it and edits apply live but do not survive a restart.
	 */
	configPath?: string
	/** Directory for anything the filler writes outside the data store. */
	dataDir?: string
	/** Forces watch-only on every chain, whatever the config says. */
	watchOnly?: boolean
	/**
	 * Where to send log records, one NDJSON line per record. Anything
	 * Writable-shaped works: `process.stdout`, a file stream, a socket, a
	 * `pino-pretty` transform, or an object that forwards into your own logger.
	 *
	 * Omit it and the filler logs nothing. A library writing to its host's stdout
	 * uninvited is the host's problem to clean up, so silence is the default.
	 */
	logger?: LogSink
	/**
	 * Gateway events for this filler.
	 *
	 * Build one with `OrderStream.create(chains)` and pass the same instance to
	 * every filler that should share it — that is how N fillers cost one chain's
	 * worth of RPC instead of N. The stream is yours: closing it is your call, and
	 * `stop()` leaves it running.
	 *
	 * Omit it and the filler builds a private stream from `config.chains` and
	 * closes it on `stop()`. Nothing is ever shared implicitly.
	 */
	orderStream?: OrderStream
	/**
	 * Hyperbridge phantom orders. Same ownership rule as `orderStream`. Omitted
	 * with a `hyperbridgeWsUrl` configured, the filler builds a private one.
	 *
	 * Bid submission is unaffected either way — it is signed with this filler's
	 * own substrate key on its own connection.
	 */
	hyperbridgeStream?: HyperbridgeStream
	/**
	 * Called after any mutation that changes the effective config, so a host can
	 * persist it however it likes.
	 */
	onConfigChange?(config: SimplexConfig): void | Promise<void>
}

/** Events a running filler emits. */
export interface SimplexEvents {
	"order:detected": { order: Order; transactionHash: string }
	"order:skipped": { orderId: string; reason: string }
	"order:filled": {
		orderId: string
		txHash?: HexString
		chainId?: number
		volumeUsd?: number
		profitUsd?: number
	}
	"order:executed": {
		orderId: string
		success: boolean
		strategy?: string
		commitment?: HexString
		txHash?: HexString
		error?: string
	}
	"order:timing": {
		orderId: string
		phase: "evaluation" | "confirmation" | "queue_wait" | "execution"
		durationSec: number
	}
	"order:filled-onchain": { commitment: HexString; filler: string; chainId: number }
	rebalance: { success: boolean; transferCount?: number; executedCount?: number; error?: string }
	activity: ActivityEvent
}

/** Internal monitor event name to public event name. */
const EVENT_MAP: Record<string, keyof SimplexEvents> = {
	newOrder: "order:detected",
	orderSkipped: "order:skipped",
	orderFilled: "order:filled",
	orderExecuted: "order:executed",
	orderTiming: "order:timing",
	orderFilledOnChain: "order:filled-onchain",
	rebalanceExecuted: "rebalance",
}

// ===========================================================================
// Views
// ===========================================================================

export interface PairView {
	index: number
	token0: string
	token1: string
	/** Per-order cap in token0 units; absent for reference-only pairs. */
	maxOrderSize?: string
	bid?: PriceCurvePoint[]
	ask?: PriceCurvePoint[]
	/** token0 === token1: the same-asset cross-chain market, ask-only below par. */
	sameToken: boolean
	/** A price feed for the USD anchor graph that never opens a market. */
	referenceOnly: boolean
	/** No static curves — priced from a Uniswap V4 pool instead. */
	venuePriced: boolean
}

export interface ChainView {
	chainId: number
	stateMachineId: string
	rpcUrls: string[]
	bundlerUrl: string
	watchOnly: boolean
}

export interface ChainInput {
	rpcUrls: string[]
	bundlerUrl: string
	watchOnly?: boolean
	/**
	 * Required for chains with no built-in confirmation curve (testnets and
	 * anything outside Ethereum, BSC, Polygon, Base, Arbitrum and Unichain).
	 * Without one, cross-chain orders sourced there would throw per order.
	 */
	confirmationPolicy?: ChainConfirmationPolicy
}

export interface SimplexStatus {
	paused: boolean
	halted: boolean
	fillerAddress: HexString
	startedAt: number
	uptimeSec: number
	chains: number[]
	watchOnly: Record<number, boolean>
	pairs: number
}

// ===========================================================================
// Controllers
// ===========================================================================

/** Trading pairs of the running engine. Every mutation binds on the next order. */
export class PairController {
	constructor(
		private runtime: FillerRuntime,
		private persist: () => void | Promise<void>,
	) {}

	private get pairs(): PairConfig[] {
		return this.runtime.config.pairs ?? []
	}

	list(): PairView[] {
		const live = this.runtime.tradingPairs ?? []
		return this.pairs.map((pair, index) => {
			const tradingPair = live[index]
			const bid = tradingPair?.bidPricePolicy?.getPoints()
			const ask = tradingPair?.askPricePolicy?.getPoints()
			return {
				index,
				token0: pair.token0,
				token1: pair.token1,
				maxOrderSize: pair.referenceOnly ? undefined : tradingPair?.maxOrderSize.toString(),
				bid,
				ask,
				sameToken: normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1),
				referenceOnly: pair.referenceOnly === true,
				venuePriced: !bid && !ask,
			}
		})
	}

	/**
	 * Opens a new market on the running engine.
	 *
	 * Validated against the same rules boot applies, so a pair accepted here is
	 * one a restart would also accept. New `assets` are registered first, because
	 * the pair's symbols have to resolve before the market can be hydrated.
	 */
	async add(pair: PairConfig, options?: { assets?: Record<string, AssetDefinition> }): Promise<PairView> {
		const { engine, tradingPairs, assetRegistry, adminStrategies, balanceTokens, config } = this.runtime
		if (!engine || !tradingPairs) {
			throw new Error("No trading engine is running — this filler was started without pairs")
		}

		const assets = options?.assets
		const nextAssets = { ...(config.assets ?? {}), ...(assets ?? {}) }
		const nextPairs = [...this.pairs, pair]
		validatePairConfigs(nextPairs, nextAssets, Boolean(config.vault?.uniswapV4?.positions?.length))

		if (assets && Object.keys(assets).length > 0) assetRegistry.addAssets(assets)
		assertPairSymbolsResolve(
			[pair],
			assetRegistry,
			this.runtime.resolvedChains.map((chain) => formatChainKey(chain.chainId)),
		)

		const tradingPair = tradingPairFrom(pair)
		// Pushes into the engine's live array — the same instance as
		// `tradingPairs`, so config.pairs indexes stay aligned.
		engine.addPair(tradingPair)
		config.pairs = nextPairs
		if (assets && Object.keys(assets).length > 0) config.assets = nextAssets

		// Track the exotic side's balances from the next refresh.
		if (normalizeSymbol(pair.token0) !== normalizeSymbol(pair.token1) && pair.referenceOnly !== true) {
			for (const chain of this.runtime.resolvedChains) {
				const chainName = formatChainKey(chain.chainId)
				const address = assetRegistry.getAddress(pair.token1, chainName)
				if (!address) continue
				const list = (balanceTokens[chainName] ??= [])
				if (!list.includes(address)) list.push(address)
			}
		}

		const pairIndex = nextPairs.length - 1
		const index = adminStrategies.reduce((max, s) => Math.max(max, s.index), -1) + 1
		const adminStrategy = adminStrategyFor(tradingPair, pairIndex, index, this.runtime.loggers.get("cli"))
		if (adminStrategy) adminStrategies.push(adminStrategy)

		await this.persist()
		return this.list()[pairIndex]
	}

	/** Closes a market. Orders already in flight against it still complete. */
	async remove(index: number): Promise<void> {
		const { engine, tradingPairs, adminStrategies, config } = this.runtime
		if (!engine || !tradingPairs) throw new Error("No trading engine is running")
		if (index < 0 || index >= this.pairs.length) throw new Error(`Unknown pair ${index}`)

		// Splices the engine's live array (same instance as tradingPairs).
		engine.removePair(tradingPairs[index])
		config.pairs = this.pairs.filter((_, i) => i !== index)

		const position = adminStrategies.findIndex((s) => s.pairIndex === index)
		if (position >= 0) adminStrategies.splice(position, 1)
		// config.pairs lost an entry, so later pairs shift down.
		for (const strategy of adminStrategies) {
			if (strategy.pairIndex > index) strategy.pairIndex -= 1
		}

		await this.persist()
	}

	/** Replaces one side's price curve. Takes effect on the next order evaluation. */
	async setCurve(index: number, side: "bid" | "ask", points: PriceCurvePoint[]): Promise<void> {
		const pair = this.livePair(index)
		const policy = side === "bid" ? pair.bidPricePolicy : pair.askPricePolicy
		if (policy) {
			// Mutating the live policy, not replacing it: the engine holds this
			// exact instance, so a swap would leave it pricing on the old curve.
			policy.replacePoints({ points })
		} else {
			const fresh = new FillerPricePolicy({ points })
			if (side === "bid") pair.bidPricePolicy = fresh
			else pair.askPricePolicy = fresh
		}
		this.writeCurveToConfig(index, side, points)
		await this.persist()
	}

	/** Closes a direction (one-sided LP). The other side keeps filling. */
	async clearCurve(index: number, side: "bid" | "ask"): Promise<void> {
		const pair = this.livePair(index)
		if (side === "bid") pair.bidPricePolicy = undefined
		else pair.askPricePolicy = undefined
		this.writeCurveToConfig(index, side, undefined)
		await this.persist()
	}

	/** Resizes the per-order cap, in token0 units. Binds on the next order. */
	async setMaxOrderSize(index: number, value: string): Promise<void> {
		const pair = this.livePair(index)
		const parsed = new Decimal(value)
		if (!parsed.isFinite() || parsed.lte(0)) {
			throw new Error(`maxOrderSize must be a positive number, got '${value}'`)
		}
		pair.maxOrderSize = parsed
		this.pairs[index].maxOrderSize = value
		await this.persist()
	}

	private livePair(index: number) {
		const pair = this.runtime.tradingPairs?.[index]
		if (!pair) throw new Error(`Unknown pair ${index}`)
		return pair
	}

	private writeCurveToConfig(index: number, side: "bid" | "ask", points: PriceCurvePoint[] | undefined): void {
		const entry = this.pairs[index]
		if (side === "bid") entry.bidPriceCurve = points
		else entry.askPriceCurve = points
	}
}

/** The chain set of the running filler. */
export class ChainController {
	constructor(
		private runtime: FillerRuntime,
		private persist: () => void | Promise<void>,
		/** Set only when this filler built its own stream and may therefore edit it. */
		private ownedStream?: OrderStream,
	) {}

	list(): ChainView[] {
		return this.runtime.resolvedChains.map((chain) => ({
			chainId: chain.chainId,
			stateMachineId: formatChainKey(chain.chainId),
			rpcUrls: [...chain.rpcUrls],
			bundlerUrl: chain.bundlerUrl ?? "",
			watchOnly: this.runtime.intentFiller.getWatchOnly()[chain.chainId] === true,
		}))
	}

	/**
	 * Adds a chain to the running filler — no restart.
	 *
	 * The endpoints are probed and the chain id read back from them before
	 * anything is mutated, so a wrong or unreachable RPC is rejected rather than
	 * half-applied. Confirmation coverage is asserted for the same reason: a
	 * chain scanning without a curve drops every cross-chain order sourced on it.
	 */
	async add(chain: ChainInput): Promise<ChainView> {
		validateRpcUrls(chain.rpcUrls)
		if (!chain.bundlerUrl?.trim()) {
			throw new Error("A bundler URL is required to submit fill UserOperations")
		}

		const [resolved] = await resolveChainConfigs([{ rpcUrls: chain.rpcUrls, bundlerUrl: chain.bundlerUrl }])
		const { chainId } = resolved
		const { configService, config, intentFiller } = this.runtime

		if (configService.getConfiguredChainIds().includes(chainId)) {
			throw new Error(`Chain ${chainId} is already configured`)
		}

		const confirmationPolicies = { ...(config.confirmationPolicies ?? {}) }
		if (chain.confirmationPolicy) confirmationPolicies[String(chainId)] = chain.confirmationPolicy
		// Throws when the chain has neither a built-in default nor a supplied curve.
		assertConfirmationCoverage(confirmationPolicies, [...configService.getConfiguredChainIds(), chainId])

		configService.addChain(resolved)
		if (chain.watchOnly) intentFiller.setWatchOnly(chainId, true)

		// The stream has to carry the chain before the monitor can match it. A
		// caller's stream is theirs, so say so rather than mutating it.
		if (!this.ownedStream) {
			throw new Error(
				`Chain ${chainId} is not in the order stream. This filler was started with a stream you own — ` +
					`add the chain to it with stream.addChain(...) before adding it here.`,
			)
		}
		await this.ownedStream.addChain({ rpcUrls: chain.rpcUrls, bundlerUrl: chain.bundlerUrl, chainId })

		try {
			await intentFiller.addChain(configService.getChainConfig(formatChainKey(chainId)))
		} catch (error) {
			// Roll back so a failed add cannot leave a chain that reads as
			// configured but is not being scanned.
			configService.removeChain(chainId)
			await this.ownedStream.removeChain(chainId).catch(() => {})
			throw error
		}

		this.runtime.resolvedChains.push(resolved)
		config.chains = [...config.chains, { rpcUrls: chain.rpcUrls, bundlerUrl: chain.bundlerUrl }]
		if (Object.keys(confirmationPolicies).length > 0) config.confirmationPolicies = confirmationPolicies
		this.syncWatchOnlyToConfig()

		await this.persist()
		return this.list().find((row) => row.chainId === chainId)!
	}

	/**
	 * Removes a chain. In-flight fills on it are drained first.
	 *
	 * Refuses while a vault or Uniswap V4 position still names the chain — those
	 * hydrate per chain at boot, and one left behind would fail the next start.
	 */
	async remove(chainId: number): Promise<void> {
		const { config, configService, intentFiller } = this.runtime
		const chainKey = formatChainKey(chainId)

		if (config.vault?.vaults?.some((vault) => vault.chain === chainKey)) {
			throw new Error(`${chainKey} still holds a vault entry — remove it from the vault treasury first`)
		}
		if (config.vault?.uniswapV4?.positions?.some((position) => position.chain === chainKey)) {
			throw new Error(`${chainKey} still holds a Uniswap V4 position — remove it first`)
		}

		const index = this.runtime.resolvedChains.findIndex((chain) => chain.chainId === chainId)
		if (index < 0) throw new Error(`Chain ${chainId} is not configured`)

		await intentFiller.removeChain(chainId)
		await this.ownedStream?.removeChain(chainId)
		configService.removeChain(chainId)
		this.runtime.chainClientManager.invalidate(chainKey)
		this.runtime.resolvedChains.splice(index, 1)
		config.chains = config.chains.filter((_, i) => i !== index)
		this.syncWatchOnlyToConfig()

		await this.persist()
	}

	/**
	 * Repoints a chain at new RPC endpoints without a restart.
	 *
	 * The new endpoints are probed first, then the cached viem clients are
	 * dropped and the scanner rebuilt from its existing cursor — so the swap
	 * leaves no gap in the scanned block range.
	 */
	async setRpcUrls(chainId: number, rpcUrls: string[]): Promise<void> {
		validateRpcUrls(rpcUrls)
		const index = this.runtime.resolvedChains.findIndex((chain) => chain.chainId === chainId)
		if (index < 0) throw new Error(`Chain ${chainId} is not configured`)

		// Probe before mutating: an endpoint answering for another chain would
		// otherwise silently feed this scanner the wrong chain's logs.
		const [probed] = await resolveChainConfigs([
			{ rpcUrls, bundlerUrl: this.runtime.resolvedChains[index].bundlerUrl ?? "" },
		])
		if (probed.chainId !== chainId) {
			throw new Error(`Those endpoints answer for chain ${probed.chainId}, not ${chainId}`)
		}

		if (!this.ownedStream) {
			throw new Error(
				`Endpoints for chain ${chainId} belong to the order stream you supplied — change them there, ` +
					`so the other fillers reading it are not repointed underneath them.`,
			)
		}

		const chainKey = formatChainKey(chainId)
		this.runtime.configService.setRpcUrls(chainId, rpcUrls)
		this.runtime.chainClientManager.invalidate(chainKey)
		// Rebuild the scan loop on the new endpoints, then the monitor's view of it.
		await this.ownedStream.removeChain(chainId)
		await this.ownedStream.addChain({ rpcUrls, chainId })
		await this.runtime.intentFiller.monitor.rebuildChain(chainId)

		this.runtime.resolvedChains[index].rpcUrls = rpcUrls
		this.runtime.config.chains[index].rpcUrls = rpcUrls
		await this.persist()
	}

	async setBundlerUrl(chainId: number, bundlerUrl: string): Promise<void> {
		if (!bundlerUrl?.trim()) throw new Error("A bundler URL is required")
		const index = this.runtime.resolvedChains.findIndex((chain) => chain.chainId === chainId)
		if (index < 0) throw new Error(`Chain ${chainId} is not configured`)

		this.runtime.configService.setBundlerUrl(chainId, bundlerUrl)
		this.runtime.resolvedChains[index].bundlerUrl = bundlerUrl
		this.runtime.config.chains[index].bundlerUrl = bundlerUrl
		await this.persist()
	}

	/** Monitor without filling. Takes effect on the next order. */
	async setWatchOnly(chainId: number, watchOnly: boolean): Promise<void> {
		this.runtime.intentFiller.setWatchOnly(chainId, watchOnly)
		this.syncWatchOnlyToConfig()
		await this.persist()
	}

	/** A global boolean is left alone — expanding it per chain changes how the config validates. */
	private syncWatchOnlyToConfig(): void {
		if (typeof this.runtime.config.simplex.watchOnly === "boolean") return
		const perChain = Object.entries(this.runtime.intentFiller.getWatchOnly())
			.filter(([, value]) => value === true)
			.map(([chainId]) => [chainId, true] as const)
		this.runtime.config.simplex.watchOnly = perChain.length > 0 ? Object.fromEntries(perChain) : undefined
	}
}

/** ERC-4626 treasury vaults the filler sources fills from and sweeps into. */
export class VaultController {
	constructor(
		private runtime: FillerRuntime,
		private persist: () => void | Promise<void>,
	) {}

	list(): VaultToml[] {
		return [...(this.runtime.config.vault?.vaults ?? [])]
	}

	/** Hydration-level validation without touching the live venue. */
	preflight(vaults: VaultToml[]): Promise<void> {
		return this.runtime.vaultPreflight(vaults)
	}

	/** Replaces the vault set and re-hydrates the shared venue. */
	async set(vaults: VaultToml[], options?: { sweepIntervalMs?: number }): Promise<void> {
		const venue = this.runtime.vaultVenue
		if (!venue) throw new Error("This filler was started without a vault treasury")

		const vaultsByChain: Record<string, VaultToml[]> = {}
		for (const row of vaults) {
			;(vaultsByChain[row.chain] ??= []).push(row)
		}
		await venue.reconfigure({
			vaultsByChain: Object.fromEntries(
				Object.entries(vaultsByChain).map(([chain, rows]) => [
					chain,
					rows.map((row) => ({
						vault: row.vault,
						threshold: row.threshold,
						minBalance: row.minBalance,
						redeemOnShutdown: row.redeemOnShutdown,
					})),
				]),
			),
			sweepIntervalMs: options?.sweepIntervalMs ?? this.runtime.config.vault?.sweepIntervalMs,
		})

		this.runtime.config.vault = {
			...this.runtime.config.vault,
			vaults,
			sweepIntervalMs: options?.sweepIntervalMs ?? this.runtime.config.vault?.sweepIntervalMs,
		}
		await this.persist()
	}

	/** Deposits wallet balance above each vault's threshold, down to its floor. */
	sweepNow(): Promise<void> {
		const venue = this.runtime.vaultVenue
		if (!venue) throw new Error("This filler was started without a vault treasury")
		return venue.sweepExcessToVault()
	}

	/** Unwinds every position back to its underlying asset. */
	redeemAll(): Promise<void> {
		const venue = this.runtime.vaultVenue
		if (!venue) throw new Error("This filler was started without a vault treasury")
		return venue.redeemAll()
	}
}

/** Symbol-to-address resolution for the configured chains. */
export class AssetController {
	constructor(
		private runtime: FillerRuntime,
		private persist: () => void | Promise<void>,
	) {}

	/**
	 * Registers custom tokens. Existing symbols cannot be redefined — repointing
	 * a symbol under a running market would silently redirect its fills.
	 */
	async add(defs: Record<string, AssetDefinition>): Promise<void> {
		this.runtime.assetRegistry.addAssets(defs)
		this.runtime.config.assets = { ...(this.runtime.config.assets ?? {}), ...defs }
		await this.persist()
	}

	/** Contract address of a symbol on a chain, or null when not deployed there. */
	resolve(symbol: string, chain: string): HexString | null {
		return this.runtime.assetRegistry.getAddress(symbol, chain)
	}
}

/** Filler wallet: balances, outbound transfers, history. */
export class WalletController {
	constructor(private runtime: FillerRuntime) {}

	get address(): HexString {
		return this.runtime.fillerAddress
	}

	/** Last refreshed snapshot; `updatedAt` is null until the first refresh lands. */
	balances(): BalanceSnapshot {
		return this.runtime.balanceProvider.getSnapshot()
	}

	/** Outbound transfer. Vault shares are redeemed to the recipient. */
	send(params: { chain: string; token: string; amount: string; to: HexString }): Promise<{
		txHash: string
		sponsored: boolean
		redeemed: boolean
	}> {
		return this.runtime.tokenSender.send(params)
	}

	/** Operator sends, vault sweeps and redeems, newest first. */
	history(limit?: number): Promise<WalletTx[]> {
		return this.runtime.data.activity.walletTxs(limit)
	}
}

/** Cross-chain inventory rebalancing. */
export class RebalanceController {
	constructor(
		private runtime: FillerRuntime,
		private persist: () => void | Promise<void>,
	) {}

	/** Replaces the rebalancing settings; trigger checks read them live. */
	async set(rebalancing: RebalancingConfig | undefined): Promise<void> {
		this.runtime.configService.setRebalancing(rebalancing)
		this.runtime.config.rebalancing = rebalancing
		await this.persist()
	}

	/** Runs a trigger check now instead of waiting for the timer. */
	trigger(): Promise<unknown> {
		const service = this.runtime.rebalancingService
		if (!service) throw new Error("This filler was started without rebalancing configured")
		return service.checkRebalanceTriggers()
	}
}

// ===========================================================================
// Simplex
// ===========================================================================

/**
 * A running Hyperbridge intent filler.
 *
 * Start one with {@link Simplex.start}, which boots every service, delegates the
 * filler account where solver selection is active, and begins scanning the
 * configured chains. The instance is the whole control surface: trading pairs,
 * chains, vaults, assets, wallet and rebalancing are reachable as controllers,
 * and everything the filler does is observable through {@link SimplexEvents}.
 *
 * Mutations apply to the live filler and bind on the next order — nothing here
 * needs a restart. They are also written back into the config object this was
 * started with, so `config()` always reflects what is actually running; pass
 * `onConfigChange` to persist that yourself.
 *
 * More than one instance can run in one process. They share no state.
 */
export class Simplex extends EventEmitter {
	readonly pairs: PairController
	readonly chains: ChainController
	readonly vaults: VaultController
	readonly assets: AssetController
	readonly wallet: WalletController
	readonly rebalancing: RebalanceController

	private logger: Logger
	private stopped = false

	private constructor(
		private runtime: FillerRuntime,
		private options: SimplexOptions,
		/** Streams this filler built for itself, and must therefore close. */
		private ownedStreams: { orders?: OrderStream; hyperbridge?: HyperbridgeStream } = {},
	) {
		super()
		this.logger = runtime.loggers.get("simplex")
		const persist = () => this.persistConfig()
		this.pairs = new PairController(runtime, persist)
		this.chains = new ChainController(runtime, persist, ownedStreams.orders)
		this.vaults = new VaultController(runtime, persist)
		this.assets = new AssetController(runtime, persist)
		this.wallet = new WalletController(runtime)
		this.rebalancing = new RebalanceController(runtime, persist)
		this.forwardEvents()
	}

	/**
	 * Boots a filler and starts it.
	 *
	 * Rejects if the config is invalid, a chain's RPCs are unreachable, or
	 * EIP-7702 delegation fails on every chain — a partially-started filler is
	 * never returned. On success the filler is already scanning.
	 */
	static async start(options: SimplexOptions): Promise<Simplex> {
		// This filler's own logging destination — not a process-wide one, so two
		// fillers in a process keep their records apart.
		const loggers = new LoggerContext({
			level: options.config.simplex.logging as LogLevel | undefined,
			sink: options.logger,
		})
		// Build private streams when none were supplied, and remember that we own
		// them: a caller's stream outlives this filler and is theirs to close.
		const orderStream = options.orderStream ?? (await OrderStream.create(options.config.chains, { loggers }))
		const ownsOrderStream = !options.orderStream

		const wsUrl = options.config.simplex.hyperbridgeWsUrl
		const hyperbridgeStream =
			options.hyperbridgeStream ?? (wsUrl ? await HyperbridgeStream.create(wsUrl, { loggers }) : undefined)
		const ownsHyperbridgeStream = !options.hyperbridgeStream && Boolean(hyperbridgeStream)

		const runtime = await bootFiller(options.config, {
			loggers,
			streams: { orders: orderStream, hyperbridge: hyperbridgeStream },
			configPath: options.configPath,
			data: options.data ?? new MemoryDataStore(),
			dataDir: options.dataDir,
			watchOnlyOverride: options.watchOnly,
		})
		return new Simplex(runtime, options, {
			orders: ownsOrderStream ? orderStream : undefined,
			hyperbridge: ownsHyperbridgeStream ? hyperbridgeStream : undefined,
		})
	}

	/** Re-emits the filler's internal events under their public names. */
	private forwardEvents(): void {
		for (const [internal, published] of Object.entries(EVENT_MAP)) {
			this.runtime.intentFiller.monitor.on(internal, (payload: Record<string, unknown>) => {
				this.emit(published, this.normalizePayload(published, payload))
			})
		}
		this.runtime.activity.on("event", (row: ActivityEvent) => this.emit("activity", row))
	}

	/** Renames the couple of internal payload keys that differ from the public shape. */
	private normalizePayload(event: keyof SimplexEvents, payload: Record<string, unknown>): unknown {
		if (event === "order:filled") {
			const { hash, ...rest } = payload
			return { ...rest, txHash: hash }
		}
		return payload
	}

	// ─── Lifecycle ──────────────────────────────────────────────────────────

	/**
	 * Stops analysing and filling new orders. Monitoring continues and in-flight
	 * fills complete; orders arriving while paused are dropped, not queued.
	 * The pause is persisted, so it survives a restart.
	 */
	async pause(): Promise<void> {
		this.runtime.intentFiller.pause()
		await this.runtime.data.state.set({ paused: true })
	}

	async resume(): Promise<void> {
		this.runtime.intentFiller.resume()
		await this.runtime.data.state.set({ paused: false })
	}

	isPaused(): boolean {
		return this.runtime.intentFiller.isPaused()
	}

	/**
	 * Drains the filler, unwinds vault positions marked `redeemOnShutdown`, and
	 * closes the data store. Idempotent, and never calls `process.exit`.
	 */
	async stop(): Promise<void> {
		if (this.stopped) return
		this.stopped = true
		await this.runtime.shutdown("Simplex.stop")
		// Only streams this filler built. One handed in by the caller keeps running
		// for whoever else is reading it.
		await this.ownedStreams.orders?.close()
		await this.ownedStreams.hyperbridge?.close()
		this.removeAllListeners()
	}

	// ─── Introspection ──────────────────────────────────────────────────────

	get address(): HexString {
		return this.runtime.fillerAddress
	}

	get startedAt(): number {
		return this.runtime.startedAt
	}

	/** The live config, including every runtime edit applied so far. */
	config(): Readonly<SimplexConfig> {
		return this.runtime.config
	}

	status(): SimplexStatus {
		return {
			paused: this.isPaused(),
			halted: this.isHalted(),
			fillerAddress: this.runtime.fillerAddress,
			startedAt: this.runtime.startedAt,
			uptimeSec: (Date.now() - this.runtime.startedAt) / 1000,
			chains: this.runtime.resolvedChains.map((chain) => chain.chainId),
			watchOnly: this.runtime.intentFiller.getWatchOnly(),
			pairs: this.runtime.config.pairs?.length ?? 0,
		}
	}

	/** Order-activity history, newest first; pass `beforeId` to page backwards. */
	activity(limit?: number, beforeId?: number): Promise<ActivityEvent[]> {
		return this.runtime.data.activity.recent(limit, beforeId)
	}

	/** Bids submitted to Hyperbridge, newest first. */
	bids(limit?: number): Promise<StoredBid[]> {
		return this.runtime.data.bids.recent(limit)
	}

	/**
	 * Bid counts by state. `pendingRetraction` is the one to watch: those are
	 * deposits still locked in the solver-selection pallet.
	 */
	bidStats(): Promise<BidStats> {
		return this.runtime.data.bids.stats()
	}

	/**
	 * Whether overfill protection has self-halted the engine. A halted filler
	 * refuses every order until {@link resetHalt}.
	 */
	isHalted(): boolean {
		return this.runtime.haltControls.some((control) => control.isHalted())
	}

	resetHalt(): void {
		for (const control of this.runtime.haltControls) control.resetHalt()
	}

	/** Restricts which order `user` addresses are processed. Omit to accept all. */
	async setAllowlist(allowlist: AllowlistConfig | undefined): Promise<void> {
		this.runtime.configService.setAllowlist(allowlist)
		this.runtime.config.allowlist = allowlist
		await this.persistConfig()
	}

	setLogLevel(level: LogLevel): void {
		this.runtime.loggers.setLevel(level)
		this.runtime.config.simplex.logging = level
	}

	/**
	 * Adds another destination for this filler's log records and returns a
	 * function that removes it. Scoped to this filler — other instances are
	 * unaffected.
	 */
	addLogSink(sink: LogSink): () => void {
		return this.runtime.loggers.addSink(sink)
	}

	// ─── Typed events ───────────────────────────────────────────────────────

	on<E extends keyof SimplexEvents>(event: E, listener: (payload: SimplexEvents[E]) => void): this
	on(event: string, listener: (...args: never[]) => void): this
	on(event: string, listener: (...args: never[]) => void): this {
		return super.on(event, listener as (...args: unknown[]) => void)
	}

	once<E extends keyof SimplexEvents>(event: E, listener: (payload: SimplexEvents[E]) => void): this
	once(event: string, listener: (...args: never[]) => void): this
	once(event: string, listener: (...args: never[]) => void): this {
		return super.once(event, listener as (...args: unknown[]) => void)
	}

	off<E extends keyof SimplexEvents>(event: E, listener: (payload: SimplexEvents[E]) => void): this
	off(event: string, listener: (...args: never[]) => void): this
	off(event: string, listener: (...args: never[]) => void): this {
		return super.off(event, listener as (...args: unknown[]) => void)
	}

	// ─── Internals ──────────────────────────────────────────────────────────

	private async persistConfig(): Promise<void> {
		if (!this.options.onConfigChange) return
		try {
			await this.options.onConfigChange(this.runtime.config)
		} catch (err) {
			// The change is already live; failing to persist it must not undo it.
			this.logger.warn({ err }, "Change applied but onConfigChange failed")
		}
	}

	/** Escape hatch for the CLI, which needs the raw runtime to serve its UI. */
	get internals(): FillerRuntime {
		return this.runtime
	}
}
