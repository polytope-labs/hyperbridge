import { isAddress } from "viem"
import { HexString } from "@hyperbridge/sdk"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
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

export interface StableStrategyConfig {
	type: "stable"
	/**
	 * Array of (amount, value) coordinates defining the BPS curve.
	 * value = basis points at that order amount
	 */
	bpsCurve: Array<{
		amount: string
		value: number
	}>
	/** Per-chain confirmation policies keyed by chain ID string. Defaults provided for ETH, BSC, Base, Arbitrum. */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
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

/** Top-level vault config: shared by the withdraw venue and the sweep timer. */
export interface VaultTomlConfig {
	vaults: VaultToml[]
	sweepIntervalMs?: number
}

export interface FxStrategyConfig {
	type: "hyperfx"
	/**
	 * Bid price curve: exotic tokens per 1 USD when the filler *buys* exotic from a user
	 * (exotic→stable leg). Should have a higher exotic-per-USD rate than the ask curve so
	 * the filler pays out fewer stablecoins per exotic token received.
	 *
	 * Optional when `[strategies.vault.uniswapV4]` lists at least one position — bid/ask
	 * are then derived from the Uniswap V4 pool after startup. Omitting only this curve
	 * (while keeping the ask) is one-sided LP: the filler stops buying exotic and only
	 * sells it, accumulating stablecoins.
	 */
	bidPriceCurve?: Array<{
		amount: string
		price: string
	}>
	/**
	 * Ask price curve: exotic tokens per 1 USD when the filler *sells* exotic to a user
	 * (stable→exotic leg). Should have a lower exotic-per-USD rate than the bid curve so
	 * the filler sends fewer exotic tokens per stablecoin received.
	 *
	 * Optional when `[strategies.vault.uniswapV4]` lists at least one position — bid/ask
	 * are then derived from the Uniswap V4 pool after startup. Omitting only this curve
	 * (while keeping the bid) is one-sided LP: the filler stops selling exotic and only
	 * buys it, accumulating the exotic token.
	 */
	askPriceCurve?: Array<{
		amount: string
		price: string
	}>
	/**
	 * Symmetric spread (basis points) around Uniswap V4 pool mid when venue pricing is used.
	 * Ignored when only static bid/ask curves apply.
	 */
	spreadBps?: number
	/** Maximum USD value per order */
	maxOrderUsd: number
	/** Map of chain identifier (e.g. "EVM-97") to exotic token contract address */
	token1: Record<string, HexString>
	/** Optional per-chain confirmation policies for cross-chain orders */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
	/** Optional on-chain liquidity funding for destination-chain outputs */
	vault?: {
		uniswapV4?: {
			positions?: UniswapV4PositionToml[]
			/**
			 * One-sided LP under pool pricing. "bid" buys exotic (accumulate exotic);
			 * "ask" sells exotic (accumulate stable). Only valid with pool pricing — i.e.
			 * no `bidPriceCurve`/`askPriceCurve` set. Omit to fill both directions.
			 */
			side?: "bid" | "ask"
		}
	}
}

export type StrategyConfig = StableStrategyConfig | FxStrategyConfig

/** Sensible defaults based on chain finality characteristics. User config overrides per-chain. */
export const DEFAULT_CONFIRMATION_POLICIES: Record<string, ChainConfirmationPolicy> = {
	"1": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 15 },
		],
	}, // Ethereum (~12s blocks, ~24s–3min)
	"56": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 3 },
		],
	}, // BNB Chain (~3s blocks, fast finality)
	"137": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 32 },
		],
	}, // Polygon (~2s blocks, milestone finality)
	"8453": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 90 },
		],
	}, // Base (~2s blocks, L2)
	"42161": {
		points: [
			{ amount: "1000", value: 8 },
			{ amount: "100000", value: 720 },
		],
	}, // Arbitrum (~0.25s blocks, L2)
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
	strategies: StrategyConfig[]
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

export function validateConfig(config: FillerTomlConfig): void {
	// Validate required fields
	// Private key is only required if not all chains are in watch-only mode
	const isWatchOnlyGlobal = config.simplex?.watchOnly === true
	const allChainsWatchOnly = isWatchOnlyGlobal

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

	if ((!config.strategies || config.strategies.length === 0) && !allChainsWatchOnly) {
		throw new Error("At least one strategy must be configured (unless all chains are in watchOnly mode)")
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
		VaultFundingPlanner.validateConfig(config.vault.vaults)
	}

	// Validate strategies
	for (const strategy of config.strategies) {
		if (!strategy.type) {
			throw new Error("Strategy type is required")
		}

		if (!["stable", "hyperfx"].includes(strategy.type)) {
			throw new Error(`Invalid strategy type: ${strategy.type}`)
		}

		if (strategy.type === "stable") {
			// Validate BPS curve
			if (!strategy.bpsCurve || !Array.isArray(strategy.bpsCurve) || strategy.bpsCurve.length < 2) {
				throw new Error("Stable strategy must have a 'bpsCurve' array with at least 2 points")
			}

			for (const point of strategy.bpsCurve) {
				if (point.amount === undefined || point.value === undefined) {
					throw new Error("Each BPS curve point must have 'amount' and 'value'")
				}
			}

			// Validate user-provided confirmation policies (defaults are always present)
			for (const [chainId, policy] of Object.entries(strategy.confirmationPolicies ?? {})) {
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
		}

		if (strategy.type === "hyperfx") {
			if (strategy.vault?.uniswapV4?.positions?.length) {
				UniswapV4FundingPlanner.validateConfig(strategy.vault.uniswapV4.positions)
			}

			const bidLen = strategy.bidPriceCurve?.length ?? 0
			const askLen = strategy.askPriceCurve?.length ?? 0

			// A single point is a valid flat curve — FillerPricePolicy returns that price at every size.
			// One-sided LP: providing only one of bid/ask restricts the filler to that direction.
			const hasAnyCurve = bidLen >= 1 || askLen >= 1
			const hasUniswapV4Positions = (strategy.vault?.uniswapV4?.positions?.length ?? 0) > 0

			if (!hasAnyCurve && !hasUniswapV4Positions) {
				throw new Error(
					"hyperfx: provide a bid and/or ask price curve, or configure [strategies.vault.uniswapV4].positions for pool-based pricing",
				)
			}

			if (strategy.spreadBps !== undefined) {
				if (!Number.isFinite(strategy.spreadBps) || strategy.spreadBps < 0 || strategy.spreadBps > 10_000) {
					throw new Error("hyperfx: 'spreadBps' must be a number between 0 and 10000")
				}
			}

			// Per-position price guard: referencePrice and maxDeviationBps are optional but
			// must be set together. A given chain may not carry conflicting guard values.
			const guardByChain: Record<string, { referencePrice: string; maxDeviationBps: number }> = {}
			for (const position of strategy.vault?.uniswapV4?.positions ?? []) {
				const hasRef = position.referencePrice !== undefined
				const hasBps = position.maxDeviationBps !== undefined
				if (hasRef !== hasBps) {
					throw new Error(
						"hyperfx: a Uniswap V4 position price guard needs both 'referencePrice' and 'maxDeviationBps', or neither",
					)
				}
				if (!hasRef) continue

				const parsedRef = Number(position.referencePrice)
				if (!Number.isFinite(parsedRef) || parsedRef <= 0) {
					throw new Error(`hyperfx: position 'referencePrice' for chain '${position.chain}' must be a positive number`)
				}
				if (
					!Number.isFinite(position.maxDeviationBps!) ||
					position.maxDeviationBps! <= 0 ||
					position.maxDeviationBps! > 10_000
				) {
					throw new Error(
						`hyperfx: position 'maxDeviationBps' for chain '${position.chain}' must be a number between 0 (exclusive) and 10000`,
					)
				}
				const existing = guardByChain[position.chain]
				if (
					existing &&
					(existing.referencePrice !== position.referencePrice || existing.maxDeviationBps !== position.maxDeviationBps)
				) {
					throw new Error(`hyperfx: conflicting price guard values for chain '${position.chain}'`)
				}
				guardByChain[position.chain] = {
					referencePrice: position.referencePrice!,
					maxDeviationBps: position.maxDeviationBps!,
				}
			}

			// One-sided LP under pool pricing: `side` enables a single direction. Only valid
			// with venue pricing and no static curves (curves express one-sided by omission).
			const side = strategy.vault?.uniswapV4?.side
			if (side !== undefined) {
				if (side !== "bid" && side !== "ask") {
					throw new Error("hyperfx: 'vault.uniswapV4.side' must be either 'bid' or 'ask'")
				}
				if (!hasUniswapV4Positions) {
					throw new Error("hyperfx: 'vault.uniswapV4.side' requires [strategies.vault.uniswapV4].positions")
				}
				if (hasAnyCurve) {
					throw new Error(
						"hyperfx: 'vault.uniswapV4.side' only applies to pool pricing; omit 'bidPriceCurve'/'askPriceCurve' (or drop one curve to do one-sided LP with static pricing)",
					)
				}
			}

			for (const point of strategy.bidPriceCurve ?? []) {
				if (point.amount === undefined || point.price === undefined) {
					throw new Error("Each FX bidPriceCurve point must have 'amount' and 'price'")
				}
			}
			for (const point of strategy.askPriceCurve ?? []) {
				if (point.amount === undefined || point.price === undefined) {
					throw new Error("Each FX askPriceCurve point must have 'amount' and 'price'")
				}
			}

			if (!strategy.maxOrderUsd) {
				throw new Error("FX strategy must have 'maxOrderUsd'")
			}

			if (!strategy.token1 || Object.keys(strategy.token1).length === 0) {
				throw new Error("FX strategy must have at least one entry in 'token1'")
			}

			if (strategy.confirmationPolicies) {
				for (const [chainId, policy] of Object.entries(strategy.confirmationPolicies)) {
					if (!policy.points || !Array.isArray(policy.points) || policy.points.length < 2) {
						throw new Error(
							`FX confirmation policy for chain ${chainId} must have a 'points' array with at least 2 points`,
						)
					}
					for (const point of policy.points) {
						if (point.amount === undefined || point.value === undefined) {
							throw new Error(
								`Each point in FX confirmation policy for chain ${chainId} must have 'amount' and 'value'`,
							)
						}
					}
				}
			}
		}
	}
}
