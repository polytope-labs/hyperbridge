import type { HexString, ERC7821Call } from "@hyperbridge/sdk"
import type { Decimal } from "decimal.js"

// =========================================================================
// Unified Funding Venue Interface
// =========================================================================

/**
 * A liquidity source that can atomically withdraw tokens and make them
 * available for order fills within a single ERC-7821 batched call.
 *
 * Implementations: UniswapV4FundingPlanner.
 */
export interface FundingVenue {
	name: string
	/** One-time startup hydration of on-chain state. */
	initialise(solver: HexString): Promise<void>
	/** Refresh live state (reserves, balances, prices). Called on-demand before withdrawal planning. */
	refresh(chain?: string): Promise<void>
	/**
	 * Plans ERC-7821 calls to withdraw `amountNeeded` of `tokenOutLower`
	 * from LP positions on `destChain`. Returns the calls and the credited
	 * amount that will become available after execution.
	 *
	 * Access is serialised per chain via a mutex so concurrent evaluations
	 * do not race on shared liquidity state.
	 */
	planWithdrawalForToken(
		destChain: string,
		solver: HexString,
		tokenOutLower: string,
		amountNeeded: bigint,
		deadlineTimestamp?: bigint,
	): Promise<FundingPlanResult>
	/**
	 * Returns the USD price (USDC/USDT) of the given exotic token on the
	 * specified chain. Uniswap V4 uses direct exotic↔stable (USDC/USDT)
	 * pools only. Uses the most-liquid qualifying pool. Returns null when
	 * no qualifying pool exists or prices have not yet been fetched.
	 *
	 * Computed on-demand from the venue's current pool state.
	 */
	getExoticTokenPrice(chain: string, exoticToken: string): Promise<Decimal | null>
}

export interface FundingPlanResult {
	calls: ERC7821Call[]
	credited: bigint
}

// =========================================================================
// Uniswap V4 Types
// =========================================================================

/**
 * Minimal per-position config. Only the tokenId is required from the user;
 * everything else (currencies, fee, ticks, liquidity) is fetched on-chain
 * during initialisation.
 */
export interface UniswapV4PositionConfig {
	tokenId: bigint
}

/**
 * User config plus data prefetched in `UniswapV4FundingPlanner.initialise`
 * (pool key, packed ticks, decimals) so `UniswapV4LiquidityState.hydrate`
 * does not repeat `getPoolAndPositionInfo` or ERC-20 `decimals` reads.
 */
export interface UniswapV4PositionInit extends UniswapV4PositionConfig {
	decimals0: number
	decimals1: number
	poolKey: {
		currency0: HexString
		currency1: HexString
		fee: number
		tickSpacing: number
		hooks: HexString
	}
	/** Packed tick bounds from `getPoolAndPositionInfo`. */
	positionInfo: bigint
}

/**
 * Runtime representation of a V4 position after on-chain hydration.
 */
export interface HydratedV4Position {
	tokenId: bigint
	positionManager: HexString
	poolManager: HexString
	currency0: HexString
	currency1: HexString
	/** ERC-20 decimals from on-chain `decimals()`; native currency uses chain native decimals (e.g. 18). */
	decimals0: number
	decimals1: number
	fee: number
	tickSpacing: number
	hooks: HexString
	tickLower: number
	tickUpper: number

	// --- live state (updated on refresh) ---
	/** Current position liquidity (uint128). */
	liquidity: bigint
	/** Available liquidity after consume() accounting for pending orders. */
	remainingLiquidity: bigint
	/** Current pool sqrtPriceX96 from slot0 — needed for amount calculations. */
	sqrtPriceX96: bigint
	/** Current tick from slot0. */
	currentTick: number
}

/**
 * Top-level Uniswap V4 funding config.
 */
export interface UniswapV4OutputFundingConfig {
	/** Chain identifier → list of position configs to source liquidity from. */
	positionsByChain: Record<string, UniswapV4PositionConfig[]>
}

// =========================================================================
// Vault Types
// =========================================================================

/**
 * A single ERC-4626 vault the filler is willing to source from (e.g. Aave's
 * stataToken for USDC). Only the vault address is needed; the underlying asset
 * and its decimals are resolved on-chain during initialisation.
 */
export interface VaultConfig {
	/** ERC-4626 vault address. */
	vault: HexString
	/**
	 * Wallet balance of the underlying asset to keep liquid for direct fills, in
	 * absolute human units (e.g. "3000"). Excess above it is swept into the
	 * vault. Omit to disable sweeping for this vault (withdraw-only).
	 */
	threshold?: string
	/**
	 * Minimum excess (absolute human units) worth sweeping; smaller excesses are
	 * skipped so gas never exceeds the value moved. Defaults to a small floor.
	 */
	minSweep?: string
}

/**
 * Runtime representation of an ERC-4626 vault after on-chain hydration.
 */
export interface HydratedVault {
	vault: HexString
	/** Underlying asset from `vault.asset()`. */
	asset: HexString
	/** Underlying asset decimals. */
	decimals: number
	/** Sweep threshold scaled to token units, or null when sweeping is disabled. */
	thresholdScaled: bigint | null
	/** Dust guard scaled to token units. */
	minSweepScaled: bigint

	// --- live state (updated on refresh) ---
	/** Solver's position in asset terms (`previewRedeem(balanceOf(solver))`). */
	positionAssets: bigint
	/** Vault's authoritative withdraw cap (`maxWithdraw(solver)`). */
	maxWithdrawable: bigint
	/** Sourceable amount after consume() accounting for pending fills this round. */
	remaining: bigint
}

/**
 * Top-level vault funding config.
 */
export interface VaultOutputFundingConfig {
	/** Chain identifier → vaults to source liquidity from. */
	vaultsByChain: Record<string, VaultConfig[]>
	/** Sweep timer cadence in ms. Defaults to 5 minutes. */
	sweepIntervalMs?: number
}

// =========================================================================
// Combined Output Funding Config
// =========================================================================

export interface OutputFundingConfig {
	uniswapV4?: UniswapV4OutputFundingConfig
	vault?: VaultOutputFundingConfig
}
