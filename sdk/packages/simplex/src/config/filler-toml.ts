import { isAddress } from "viem"
import { HexString } from "@hyperbridge/sdk"
import { parseChainKey } from "@/config/interpolated-curve"
import { USD_STABLE_SYMBOLS, validateAssetDefinitions, type AssetDefinition } from "@/config/asset-registry"
import { validatePairConfigs, type PairConfig } from "@/config/pairs"
import { validateUniswapV4Positions } from "@/config/v4-validate"
import { validateVaultToml } from "@/config/vault-validate"
import type { SignerConfig } from "@/services/wallet"
import type { UserProvidedChainConfig, AllowlistConfig } from "@/services/FillerConfigService"
import type { PaymasterKeeperConfig } from "@/services/PaymasterKeeperService"

export interface ChainConfirmationPolicy {
	/**
	 * Array of (amount, value) coordinates defining the confirmation curve.
	 * value = number of confirmations at that order amount
	 */
	points: Array<{
		amount: string
		value: number
	}>
}

/** TOML row for a Uniswap V4 position; only chain + tokenId required. */
export interface UniswapV4PositionToml {
	chain: string
	tokenId: string // bigint as string in TOML
	/**
	 * Optional price guard. When set (alongside `maxDeviationBps`), the filler rejects
	 * orders whenever the pool quote on this chain drifts more than `maxDeviationBps`
	 * from this static reference price (exotic per USD, same units as the bid/ask curves).
	 * Guards against a manipulated, stale, or thin pool.
	 */
	referencePrice?: string
	/** Tolerance in basis points for the price guard. Required when `referencePrice` is set. */
	maxDeviationBps?: number
}

/**
 * TOML row for an ERC-4626 vault entry. `threshold` (absolute human units) is the
 * high-water mark that triggers a sweep down to `minBalance`; omit both for
 * withdraw-only sourcing.
 */
export interface VaultToml {
	chain: string
	vault: HexString
	threshold?: string
	minBalance?: string
	redeemOnShutdown?: boolean
}

/**
 * Top-level `[vault]` config — every on-chain liquidity venue in one place:
 * ERC-4626 treasury vaults (withdraw sourcing + threshold sweeping) and
 * Uniswap V4 LP positions (exotic funding + pool-based pricing).
 */
export interface VaultTomlConfig {
	vaults?: VaultToml[]
	sweepIntervalMs?: number
	uniswapV4?: {
		positions?: UniswapV4PositionToml[]
		/**
		 * One-sided LP under pool pricing. "bid" only buys token1; "ask" only
		 * sells it. Only valid when no pair has static curves (curves express
		 * one-sidedness by omission). Omit to fill both directions.
		 */
		side?: "bid" | "ask"
		/**
		 * Slippage tolerance (basis points) for LP redemptions and the symmetric
		 * spread around pool mid when venue pricing is used. Default 50.
		 */
		spreadBps?: number
	}
}

export interface QueueConfig {
	maxRechecks: number
	recheckDelayMs: number
}

export interface RebalancingConfig {
	triggerPercentage: number
	baseBalances: {
		USDC?: Record<string, string>
		USDT?: Record<string, string>
	}
}

export interface BinanceConfig {
	apiKey: string
	apiSecret: string
	basePath?: string
	timeout?: number
	depositTimeoutMs?: number
	pollIntervalMs?: number
	withdrawTimeoutMs?: number
}

export interface FillerTomlConfig {
	/**
	 * Optional asset-registry escape hatch: symbol → { chain → address }. Only
	 * needed for assets the built-in registry does not ship, or to override a
	 * shipped address for a deployment. Shipped symbols: USDC, USDT, DAI, CNGN
	 * (SDK chain registry) + curated ZARP, EURC, XSGD, TRYB.
	 */
	assets?: Record<string, AssetDefinition>
	/**
	 * Trading pairs — the entire trading configuration. Each pair prices
	 * `token1` in units of `token0` via its own bid/ask curves and carries a
	 * per-order `maxOrderSize` cap; a same-token pair (token0 == token1) is the
	 * same-asset cross-chain market. Required unless running watch-only.
	 */
	pairs?: PairConfig[]
	/**
	 * Per-chain confirmation policies for cross-chain orders, keyed by chain id.
	 * Merged over built-in defaults (ETH, BSC, Polygon, Base, Arbitrum,
	 * Unichain); every configured chain must be covered or startup fails. The
	 * curve amount axis is the order's USD value, derived from the pair curves
	 * via the USD anchors.
	 */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
	simplex: {
		// The signer is optional to keep the watch-only mode compatible
		signer?: SignerConfig
		maxConcurrentOrders: number
		queue: QueueConfig
		logging?: string
		watchOnly?: boolean | Record<string, boolean>
		substratePrivateKey: string
		hyperbridgeWsUrl: string
		entryPointAddress?: string
		solverAccountContractAddress?: string
		/** Target gas units for EntryPoint deposits per chain. Defaults to 3,000,000. */
		targetGasUnits?: number
		/** Gas fee bump (percentages added to base gasPrice). Defaults: priority=8%, max=10%. */
		gasFeeBump?: {
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
		}
		/**
		 * Overfill protection knobs. Defaults: maxOverfillBps=500, maxConsecutiveClamps=3.
		 * `maxOverfillBps` clamps the per-leg output ceiling on every strategy.
		 * `maxConsecutiveClamps` only halts FXFiller, and only when the clamped legs
		 * were priced by an on-chain venue (e.g. Uniswap V4). Offline-curve clamps warn
		 * but never halt.
		 */
		overfillProtection?: {
			maxOverfillBps?: number
			maxConsecutiveClamps?: number
		}
	}
	chains: UserProvidedChainConfig[]
	rebalancing?: RebalancingConfig
	binance?: BinanceConfig
	/** Filler-wide vault config: stablecoin sourcing for fills + threshold sweeping. */
	vault?: VaultTomlConfig
	/** Restricts order processing to listed user addresses. Omit to accept all users. */
	allowlist?: AllowlistConfig
	/** SimplexPaymaster fee-recycling keeper (`paymaster-keeper` subcommand). */
	keeper?: PaymasterKeeperConfig
}

export function validateConfig(config: FillerTomlConfig, cliWatchOnly = false): void {
	// The [[strategies]] array was removed when the pair engine subsumed the
	// stable strategy — fail loudly so stale configs are migrated, not ignored.
	if ("strategies" in config) {
		throw new Error(
			"[[strategies]] was removed — declare top-level [[pairs]] instead (a same-token pair like USDC/USDC with an ask curve below par replaces the stable strategy; engine settings moved to [confirmationPolicies] and [vault.uniswapV4])",
		)
	}

	// Private key is only required if not all chains are in watch-only mode.
	// The --watch-only CLI flag forces global watch-only, so honour it here too
	// (otherwise the flag's own config would still trip the signer requirement).
	const allChainsWatchOnly = cliWatchOnly || config.simplex?.watchOnly === true

	const signer = config.simplex?.signer

	if (!signer && !allChainsWatchOnly) {
		throw new Error("Signer configuration is required via [simplex.signer]")
	}

	if (!config.simplex?.substratePrivateKey) {
		throw new Error("simplex.substratePrivateKey is required")
	}

	if (!config.simplex?.hyperbridgeWsUrl) {
		throw new Error("simplex.hyperbridgeWsUrl is required")
	}

	if (!config.chains || config.chains.length === 0) {
		throw new Error("At least one chain must be configured")
	}

	// Validate chain configurations
	for (const chain of config.chains) {
		if (!Array.isArray(chain.rpcUrls) || chain.rpcUrls.length === 0 || chain.rpcUrls.some((u) => !u)) {
			throw new Error("Each chain configuration must have rpcUrls (a non-empty array of strings)")
		}
		if (!chain.bundlerUrl) {
			throw new Error("Each chain configuration must have bundlerUrl")
		}
	}

	// Validate allowlist addresses (when present)
	if (config.allowlist) {
		for (const user of config.allowlist.users ?? []) {
			if (!isAddress(user)) {
				throw new Error(`allowlist.users contains an invalid address: ${user}`)
			}
		}
		for (const [chain, users] of Object.entries(config.allowlist.bySource ?? {})) {
			if (!Array.isArray(users)) {
				throw new Error(`allowlist.bySource."${chain}" must be an array of addresses`)
			}
			for (const user of users) {
				if (!isAddress(user)) {
					throw new Error(`allowlist.bySource."${chain}" contains an invalid address: ${user}`)
				}
			}
		}
	}

	if (config.vault?.vaults?.length) {
		validateVaultToml(config.vault.vaults)
	}

	// Asset registry and trading pairs — the entire trading configuration.
	if (config.assets) {
		validateAssetDefinitions(config.assets)
	}
	const hasPairs = (config.pairs?.length ?? 0) > 0
	if (!hasPairs && !allChainsWatchOnly) {
		throw new Error("At least one [[pairs]] entry must be configured (unless all chains are in watchOnly mode)")
	}
	const hasVenuePricing = (config.vault?.uniswapV4?.positions?.length ?? 0) > 0
	if (hasPairs) {
		validatePairConfigs(config.pairs!, config.assets, hasVenuePricing)
		// The engine only venue-prices pairs whose quote side is a dollar (a
		// pool's USD quote must invert into a pair rate). validatePairConfigs
		// accepts curve-less pairs whenever venues exist, so enforce the
		// stable-quote rule here — at the gate, not first at boot.
		if (hasVenuePricing) {
			for (const pair of config.pairs!) {
				const curveless = (pair.bidPriceCurve?.length ?? 0) === 0 && (pair.askPriceCurve?.length ?? 0) === 0
				if (curveless && !USD_STABLE_SYMBOLS.has(pair.token0.trim().toUpperCase())) {
					throw new Error(
						`pairs.${pair.token0}/${pair.token1}: venue pricing needs a USD-stable token0 — add bid/ask curves or quote against USDC/USDT/DAI`,
					)
				}
			}
		}
	}

	// Per-chain confirmation policies (merged over built-in defaults at startup).
	for (const [chainId, policy] of Object.entries(config.confirmationPolicies ?? {})) {
		if (parseChainKey(chainId) === null) {
			throw new Error(
				`Confirmation policy key '${chainId}' is not a chain id — write [confirmationPolicies."EVM-<id>"] or [confirmationPolicies."<id>"]`,
			)
		}
		if (!policy.points || !Array.isArray(policy.points) || policy.points.length < 2) {
			throw new Error(
				`Confirmation policy for chain ${chainId} must have a 'points' array with at least 2 points`,
			)
		}
		for (const point of policy.points) {
			if (point.amount === undefined || point.value === undefined) {
				throw new Error(
					`Each point in confirmation policy for chain ${chainId} must have 'amount' and 'value'`,
				)
			}
		}
	}

	// Uniswap V4 venue config ([vault.uniswapV4]): positions, price guards,
	// one-sided switch, redemption slippage.
	const uniswapV4 = config.vault?.uniswapV4
	if (uniswapV4?.positions?.length) {
		validateUniswapV4Positions(uniswapV4.positions)
	}

	if (uniswapV4?.spreadBps !== undefined) {
		if (!Number.isFinite(uniswapV4.spreadBps) || uniswapV4.spreadBps < 0 || uniswapV4.spreadBps > 10_000) {
			throw new Error("vault.uniswapV4: 'spreadBps' must be a number between 0 and 10000")
		}
	}

	// Per-position price guard: referencePrice and maxDeviationBps are optional but
	// must be set together. A given chain may not carry conflicting guard values.
	const guardByChain: Record<string, { referencePrice: string; maxDeviationBps: number }> = {}
	for (const position of uniswapV4?.positions ?? []) {
		const hasRef = position.referencePrice !== undefined
		const hasBps = position.maxDeviationBps !== undefined
		if (hasRef !== hasBps) {
			throw new Error(
				"vault.uniswapV4: a position price guard needs both 'referencePrice' and 'maxDeviationBps', or neither",
			)
		}
		if (!hasRef) continue

		const parsedRef = Number(position.referencePrice)
		if (!Number.isFinite(parsedRef) || parsedRef <= 0) {
			throw new Error(
				`vault.uniswapV4: position 'referencePrice' for chain '${position.chain}' must be a positive number`,
			)
		}
		if (
			!Number.isFinite(position.maxDeviationBps!) ||
			position.maxDeviationBps! <= 0 ||
			position.maxDeviationBps! > 10_000
		) {
			throw new Error(
				`vault.uniswapV4: position 'maxDeviationBps' for chain '${position.chain}' must be a number between 0 (exclusive) and 10000`,
			)
		}
		const existing = guardByChain[position.chain]
		if (
			existing &&
			(existing.referencePrice !== position.referencePrice || existing.maxDeviationBps !== position.maxDeviationBps)
		) {
			throw new Error(`vault.uniswapV4: conflicting price guard values for chain '${position.chain}'`)
		}
		guardByChain[position.chain] = {
			referencePrice: position.referencePrice!,
			maxDeviationBps: position.maxDeviationBps!,
		}
	}

	// One-sided LP under pool pricing: `side` enables a single direction. Only valid
	// with venue pricing and no static curves (curves express one-sided by omission).
	const side = uniswapV4?.side
	if (side !== undefined) {
		if (side !== "bid" && side !== "ask") {
			throw new Error("vault.uniswapV4: 'side' must be either 'bid' or 'ask'")
		}
		if (!hasVenuePricing) {
			throw new Error("vault.uniswapV4: 'side' requires [vault.uniswapV4].positions")
		}
		const anyCurves = (config.pairs ?? []).some(
			(p) => (p.bidPriceCurve?.length ?? 0) > 0 || (p.askPriceCurve?.length ?? 0) > 0,
		)
		if (anyCurves) {
			throw new Error(
				"vault.uniswapV4: 'side' only applies to pool pricing; omit the pair price curves (or drop one curve to do one-sided LP with static pricing)",
			)
		}
	}
}
