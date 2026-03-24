import type { HexString } from "@hyperbridge/sdk"

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

export interface OutputFundingConfig {
	aerodrome?: AerodromeOutputFundingConfig
}
