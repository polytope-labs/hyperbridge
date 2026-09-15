import type { HexString, ERC7821Call } from "@hyperbridge/sdk"

// =========================================================================
// Unified Funding Venue Interface
// =========================================================================

/**
 * A liquidity source that can atomically withdraw tokens and make them
 * available for order fills within a single ERC-7821 batched call.
 *
 * Implementations: VaultFundingPlanner.
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
	 * Wallet balance of `tokenLower` the fill must keep liquid and never source
	 * from — the vault's `minBalance` floor, reserved for gas/paymaster paid in
	 * this token. Returns 0 when the venue has no reserve for the token, for
	 * instance when no vault is configured.
	 */
	walletReserveForToken(chain: string, tokenLower: string): bigint
}

export interface FundingPlanResult {
	calls: ERC7821Call[]
	credited: bigint
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
	 * High-water trigger, in absolute human units (e.g. "5000"). A sweep fires
	 * only once the wallet balance reaches this, then deposits everything down to
	 * `minBalance`. Omit to disable sweeping for this vault (withdraw-only).
	 */
	threshold?: string
	/**
	 * Floor of the underlying to always keep liquid in the wallet, in absolute
	 * human units (e.g. "3000") — covers direct fills and gas/paymaster paid in
	 * this token. The sweep never drops below it. Required when `threshold` is
	 * set, and must be strictly less than it.
	 */
	minBalance?: string
	/**
	 * Whether to redeem this vault's position back to the underlying asset on
	 * shutdown. Defaults to false (the position is kept across restarts). Set
	 * true to unwind it back to the underlying on stop.
	 */
	redeemOnShutdown?: boolean
}

/**
 * Runtime representation of an ERC-4626 vault after on-chain hydration.
 */
export interface HydratedVault {
	vault: HexString
	/** Underlying asset from `vault.asset()`. */
	asset: HexString
	/** Underlying asset symbol, resolved on-chain during hydration. */
	symbol: string
	/** Underlying asset decimals. */
	decimals: number
	/** High-water sweep trigger scaled to token units, or null when sweeping is disabled. */
	thresholdScaled: bigint | null
	/** Wallet floor the sweep deposits down to, scaled to token units. */
	minBalanceScaled: bigint
	/** Whether shutdown redeems this position back to the underlying asset. */
	redeemOnShutdown: boolean

	// --- live state (updated on refresh) ---
	/** Solver's position in asset terms (`previewRedeem(balanceOf(solver))`). */
	positionAssets: bigint
	/** Vault's authoritative withdraw cap (`maxWithdraw(solver)`). */
	maxWithdrawable: bigint
	/**
	 * Vault's deposit cap for the solver (`maxDeposit(solver)`). Zero while the vault refuses new
	 * capital — a streaming-yield vault mid-tranche, a supply cap, a paused market — which is when a
	 * sweep has nothing it can do.
	 */
	maxDeposit: bigint
	/** Sourceable amount after consume() accounting for pending fills this round. */
	remaining: bigint
}

/**
 * Read-only vault balance projection consumed by operator-facing services.
 * Amounts remain in base units so formatting and aggregation happen once at
 * the API boundary, with no loss of precision inside the funding domain.
 */
export interface VaultBalancePosition {
	chain: string
	vault: HexString
	asset: HexString
	symbol: string
	decimals: number
	/** Solver-owned vault position in underlying-asset terms. */
	positionAssets: bigint
	/** Immediately withdrawable amount after pending-fill reservations. */
	availableAssets: bigint
	/** Underlying amount deliberately kept liquid in the solver wallet. */
	walletReserve: bigint
	/** False while `maxDeposit(solver)` is zero, so a dashboard can say why sweeps are idle. */
	acceptsDeposits: boolean
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
	vault?: VaultOutputFundingConfig
}
