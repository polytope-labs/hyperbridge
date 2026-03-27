import type { HexString, ERC7821Call } from "@hyperbridge/sdk"
import type { Decimal } from "decimal.js"

// =========================================================================
// Unified Funding Venue Interface
// =========================================================================

/**
 * A liquidity source that can atomically withdraw tokens and make them
 * available for order fills within a single ERC-7821 batched call.
 *
 * Implementations: AerodromeFundingPlanner, UniswapV4FundingPlanner.
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
// Aerodrome Types
// =========================================================================

/**
 * Minimal per-pool config.  Everything else (token0, token1, stable) is read
 * from the pair contract at initialisation time.
 *
 * If `gauge` is provided the LP is assumed to be staked there; otherwise we
 * look for LP directly in the solver wallet.
 */
export interface AerodromePoolConfig {
	pair: HexString
	/** If present, LP is staked in this gauge and must be withdrawn first. */
	gauge?: HexString
}

/**
 * Runtime representation of a pool after on-chain hydration.
 * Created once at startup / refresh — never serialised to config.
 */
export interface HydratedPool {
	pair: HexString
	token0: HexString
	token1: HexString
	stable: boolean
	router: HexString
	gauge?: HexString

	// --- live state (updated on refresh) ---
	reserve0: bigint
	reserve1: bigint
	totalSupply: bigint
	/** Wallet LP + gauge-staked LP that can still be scheduled to burn. */
	remainingLp: bigint
	/** LP sitting in the solver wallet right now (subset of remainingLp). */
	walletLp: bigint
	/** LP staked in the gauge (subset of remainingLp). Zero when no gauge. */
	gaugeLp: bigint
}

/**
 * Top-level Aerodrome funding config.
 */
export interface AerodromeOutputFundingConfig {
	/** Chain identifier → list of pool configs to source liquidity from. */
	poolsByChain: Record<string, AerodromePoolConfig[]>
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
// Combined Output Funding Config
// =========================================================================

export interface OutputFundingConfig {
	aerodrome?: AerodromeOutputFundingConfig
	uniswapV4?: UniswapV4OutputFundingConfig
}
