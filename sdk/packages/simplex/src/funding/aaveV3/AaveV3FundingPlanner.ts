import { AAVE_V3_POOL_ABI } from "@/config/abis/AaveV3Pool"
import type { AaveV3OutputFundingConfig, FundingPlanResult, FundingVenue } from "@/funding/types"
import { AaveV3LiquidityState } from "@/funding/aaveV3/AaveV3LiquidityState"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import type { ERC7821Call, HexString } from "@hyperbridge/sdk"
import { Mutex } from "async-mutex"
import type { Decimal } from "decimal.js"
import { encodeFunctionData } from "viem"

const logger = getLogger("aavev3-funding")

/**
 * Funding venue that sources output tokens by withdrawing the solver's own
 * supplied liquidity from Aave V3 (`Pool.withdraw`). Withdrawal is 1:1 with no
 * slippage: burning aTokens returns the underlying asset to the solver.
 *
 * Sourcing is one-sided per token — tokens not listed in the config yield a
 * no-op plan so the caller falls back to the wallet balance or another venue.
 *
 * Aave reserves here are stablecoins (USDC/USDT), so this venue does not price
 * exotic tokens: {@link getExoticTokenPrice} always returns null.
 */
export class AaveV3FundingPlanner implements FundingVenue {
	name = "AaveV3"
	private stateByChain = new Map<string, AaveV3LiquidityState>()
	private mutexByChain = new Map<string, Mutex>()

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly config: AaveV3OutputFundingConfig,
		private readonly configService: FillerConfigService,
	) {}

	/**
	 * Validates raw TOML reserve entries before constructing the planner.
	 * Throws on missing/invalid required fields.
	 */
	static validateConfig(reserves: { chain?: string; asset?: string }[]): void {
		for (const r of reserves) {
			if (!r.chain?.trim()) {
				throw new Error("Each AaveV3 reserve must have a non-empty 'chain' (e.g. EVM-8453)")
			}
			if (!r.asset?.trim()) {
				throw new Error("Each AaveV3 reserve must include an 'asset' address")
			}
		}
	}

	// =========================================================================
	// Lifecycle (FundingVenue)
	// =========================================================================

	async initialise(solver: HexString): Promise<void> {
		for (const [chain, reserves] of Object.entries(this.config.reservesByChain)) {
			const pool = this.configService.getAaveV3PoolAddress(chain)
			if (!pool || pool === "0x") {
				throw new Error(`AaveV3 Pool not configured for chain ${chain}`)
			}

			logger.info({ chain, pool, reserveCount: reserves.length, solver }, "AaveV3 initialising chain")

			const state = new AaveV3LiquidityState(chain, reserves, pool, solver, this.clientManager)
			await state.hydrate()
			this.stateByChain.set(chain, state)
			this.mutexByChain.set(chain, new Mutex())
		}
	}

	async refresh(chain?: string): Promise<void> {
		if (chain) {
			const state = this.stateByChain.get(chain)
			if (state) await state.refresh()
		} else {
			await Promise.all(Array.from(this.stateByChain.values()).map((s) => s.refresh()))
		}
	}

	// =========================================================================
	// Pricing (FundingVenue)
	// =========================================================================

	/** Aave reserves are stablecoins; this venue does not price exotic tokens. */
	async getExoticTokenPrice(_chain: string, _exoticToken: string): Promise<Decimal | null> {
		return null
	}

	// =========================================================================
	// Planning (FundingVenue)
	// =========================================================================

	/**
	 * Produces a single `Pool.withdraw` ERC-7821 call that sends up to
	 * `amountNeeded` of `tokenOutLower` to the solver, capped by the sourceable
	 * amount (solver aToken balance and the pool's withdrawable reserve).
	 * Returns a no-op when the token is not an Aave reserve on this chain.
	 */
	async planWithdrawalForToken(
		destChain: string,
		solver: HexString,
		tokenOutLower: string,
		amountNeeded: bigint,
		_deadlineTimestamp?: bigint,
	): Promise<FundingPlanResult> {
		const noopResult: FundingPlanResult = { calls: [], credited: 0n }

		if (amountNeeded <= 0n) return noopResult

		const state = this.stateByChain.get(destChain)
		if (!state || !state.isHydrated()) return noopResult

		const mutex = this.mutexByChain.get(destChain)!
		return mutex.runExclusive(async () => {
			await state.refresh()

			const tokenNeed = tokenOutLower.toLowerCase()
			const reserve = state.reserveForToken(tokenNeed)
			if (!reserve) return noopResult

			const available = state.remaining(reserve.asset)
			if (available <= 0n) return noopResult

			const amount = amountNeeded < available ? amountNeeded : available
			const pool = this.configService.getAaveV3PoolAddress(destChain)

			const call: ERC7821Call = {
				target: pool,
				value: 0n,
				data: encodeFunctionData({
					abi: AAVE_V3_POOL_ABI,
					functionName: "withdraw",
					args: [reserve.asset, amount, solver],
				}) as HexString,
			}

			state.consume(reserve.asset, amount)

			logger.debug(
				{
					destChain,
					asset: reserve.asset,
					amountNeeded: amountNeeded.toString(),
					available: available.toString(),
					credited: amount.toString(),
				},
				"AaveV3 funding planned",
			)

			return { calls: [call], credited: amount }
		})
	}
}
