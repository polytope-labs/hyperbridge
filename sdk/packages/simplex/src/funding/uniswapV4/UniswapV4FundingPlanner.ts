import { UNISWAP_V4_POSITION_MANAGER_ABI } from "@/config/abis/UniswapV4"
import type {
	FundingVenue,
	HydratedV4Position,
	UniswapV4OutputFundingConfig,
	UniswapV4PositionInit,
} from "@/funding/types"
import { UniswapV4LiquidityState } from "@/funding/uniswapV4/UniswapV4LiquidityState"
import { chainIdFromIdentifier, fetchPoolCurrencyDecimals } from "@/funding/uniswapV4/v4PoolCurrency"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import type { ERC7821Call, HexString } from "@hyperbridge/sdk"
import { Percent } from "@uniswap/sdk-core"
import { Position as V4Position, V4PositionManager } from "@uniswap/v4-sdk"
import type { RemoveLiquidityOptions } from "@uniswap/v4-sdk"
import JSBI from "jsbi"
import { encodeFunctionData } from "viem"

const logger = getLogger("uniswapv4-funding")

/** Slippage tolerance for remove-liquidity operations (0.30%). */
const SLIPPAGE_TOLERANCE = new Percent(30, 10_000) // 0.30%

/**
 * Funding venue that sources output tokens by removing liquidity from
 * Uniswap V4 concentrated-liquidity positions (ERC-721 NFTs).
 *
 * Uses the official `@uniswap/v4-sdk` to:
 *   - Construct `Pool` and `Position` objects from on-chain state
 *   - Compute expected token amounts for a given liquidity decrease
 *   - Generate `removeCallParameters` calldata (DECREASE_LIQUIDITY + TAKE_PAIR)
 *
 * The resulting calldata targets `PositionManager.multicall()` and is
 * wrapped into an ERC-7821 call for batched UserOp execution.
 */
export class UniswapV4FundingPlanner implements FundingVenue {
	name = "UniswapV4"
	/** Long-lived state per chain, keyed by chain identifier. */
	private stateByChain = new Map<string, UniswapV4LiquidityState>()

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly config: UniswapV4OutputFundingConfig,
		private readonly configService: FillerConfigService,
	) {}

	/**
	 * Validates raw TOML position entries before constructing the planner.
	 * Throws on missing/invalid required fields.
	 */
	static validateConfig(positions: { chain?: string; tokenId?: string }[]): void {
		for (const pos of positions) {
			if (!pos.chain?.trim()) {
				throw new Error("Each UniswapV4 outputFunding position must have a non-empty 'chain' (e.g. EVM-8453)")
			}
			if (!pos.tokenId) {
				throw new Error("Each UniswapV4 position must include a 'tokenId'")
			}
		}
	}

	// =========================================================================
	// Lifecycle (FundingVenue)
	// =========================================================================

	/**
	 * Call once at startup. Hydrates every configured chain's positions by
	 * reading pool keys, tick ranges, liquidity, and slot0 via StateView.
	 */
	async initialise(solver: HexString): Promise<void> {
		for (const [chain, positions] of Object.entries(this.config.positionsByChain)) {
			logger.info({ chain, positionCount: positions.length, solver }, "UniswapV4 initialising chain")
			const positionManager = this.configService.getUniswapV4PositionManagerAddress(chain)
			const poolManager = this.configService.getUniswapV4PoolManagerAddress(chain)
			const stateView = this.configService.getUniswapV4StateViewAddress(chain)

			if (!positionManager || positionManager === "0x") {
				throw new Error(`UniswapV4 PositionManager not configured for chain ${chain}`)
			}
			if (!poolManager || poolManager === "0x") {
				throw new Error(`UniswapV4 PoolManager not configured for chain ${chain}`)
			}
			if (!stateView || stateView === "0x") {
				throw new Error(`UniswapV4 StateView not configured for chain ${chain}`)
			}

			logger.info({ chain, positionManager, poolManager, stateView }, "UniswapV4 addresses resolved")

			const client = this.clientManager.getPublicClient(chain)
			const chainId = chainIdFromIdentifier(chain)

			const positionInits: UniswapV4PositionInit[] = []
			for (const cfg of positions) {
				const [poolKey, positionInfo] = (await client.readContract({
					address: positionManager,
					abi: UNISWAP_V4_POSITION_MANAGER_ABI,
					functionName: "getPoolAndPositionInfo",
					args: [cfg.tokenId],
				})) as [
					{
						currency0: HexString
						currency1: HexString
						fee: number
						tickSpacing: number
						hooks: HexString
					},
					bigint,
				]

				const [decimals0, decimals1] = await Promise.all([
					fetchPoolCurrencyDecimals(client, chainId, poolKey.currency0),
					fetchPoolCurrencyDecimals(client, chainId, poolKey.currency1),
				])
				positionInits.push({
					tokenId: cfg.tokenId,
					decimals0,
					decimals1,
					poolKey,
					positionInfo,
				})
			}

			const state = new UniswapV4LiquidityState(
				chain,
				positionInits,
				positionManager,
				poolManager,
				stateView,
				solver,
				this.clientManager,
			)
			await state.hydrate()
			this.stateByChain.set(chain, state)
		}
	}

	/**
	 * Refresh live data (liquidity, slot0) for one or all chains.
	 */
	async refresh(chain?: string): Promise<void> {
		if (chain) {
			const state = this.stateByChain.get(chain)
			if (state) await state.refresh()
		} else {
			await Promise.all(Array.from(this.stateByChain.values()).map((s) => s.refresh()))
		}
	}

	// =========================================================================
	// Planning (FundingVenue)
	// =========================================================================

	/**
	 * Produces ERC-7821 calls to decrease liquidity and take tokens from
	 * Uniswap V4 positions so that at least `amountNeeded` of `tokenOutLower`
	 * is credited to the solver.
	 *
	 * Uses the SDK's `Position` class for amount calculations and
	 * `V4PositionManager.removeCallParameters()` for calldata generation.
	 * Can aggregate across multiple positions if one doesn't cover the full deficit.
	 */
	async planWithdrawalForToken(
		destChain: string,
		solver: HexString,
		tokenOutLower: string,
		amountNeeded: bigint,
	): Promise<{ calls: ERC7821Call[]; credited: bigint }> {
		logger.info(
			{
				destChain,
				solver,
				tokenOutLower,
				amountNeeded: amountNeeded.toString(),
			},
			"UniswapV4 planWithdrawalForToken called",
		)

		if (amountNeeded <= 0n) return { calls: [], credited: 0n }

		const state = this.stateByChain.get(destChain)
		if (!state || !state.isHydrated()) {
			logger.info(
				{ destChain, hasState: !!state, isHydrated: state?.isHydrated() },
				"UniswapV4 no state or not hydrated",
			)
			return { calls: [], credited: 0n }
		}

		const tokenNeed = tokenOutLower.toLowerCase()
		const candidates = state
			.positionsForToken(tokenNeed)
			.sort((a, b) => (b.remainingLiquidity > a.remainingLiquidity ? 1 : -1))

		logger.info(
			{
				tokenNeed,
				candidateCount: candidates.length,
				candidates: candidates.map((c) => ({
					tokenId: c.tokenId.toString(),
					remainingLiquidity: c.remainingLiquidity.toString(),
					currency0: c.currency0,
					currency1: c.currency1,
				})),
			},
			"UniswapV4 candidates found",
		)

		let remaining = amountNeeded
		const allCalls: ERC7821Call[] = []
		let totalCredited = 0n

		for (const pos of candidates) {
			if (remaining <= 0n) break

			const availLiq = state.remaining(pos.tokenId)
			if (availLiq === 0n) continue

			const isToken0 = pos.currency0.toLowerCase() === tokenNeed

			// Bump the target by 0.5% so the V4 withdrawal overshoots the
			// exact amount needed.  The on-chain DECREASE_LIQUIDITY may yield
			// slightly less than the SDK's theoretical calculation due to
			// rounding, price movement, or slippage.  By requesting 0.5% more
			// liquidity we ensure the solver's wallet balance after withdrawal
			// always covers the committed output amount.  The small surplus
			// stays in the solver's wallet.
			const bufferedRemaining = remaining + (remaining * 5n) / 1000n

			// Use binary search to find the minimal liquidity that covers the buffered target
			const neededLiq = this.findLiquidityForDeficit(state, pos, isToken0, bufferedRemaining)
			if (neededLiq <= 0n) continue

			const cappedLiq = neededLiq > availLiq ? availLiq : neededLiq

			// Build SDK Position to compute expected amounts
			const sdkPosition = state.buildSdkPosition(pos.tokenId, cappedLiq)
			if (!sdkPosition) continue

			// The SDK Position computes amounts for the given liquidity
			const amount0 = BigInt(sdkPosition.amount0.quotient.toString())
			const amount1 = BigInt(sdkPosition.amount1.quotient.toString())
			const credit = isToken0 ? amount0 : amount1

			logger.info(
				{
					tokenId: pos.tokenId.toString(),
					isToken0,
					sqrtPriceX96: pos.sqrtPriceX96.toString(),
					neededLiq: neededLiq.toString(),
					availLiq: availLiq.toString(),
					cappedLiq: cappedLiq.toString(),
					amount0: amount0.toString(),
					amount1: amount1.toString(),
					credit: credit.toString(),
					requestedDeficit: remaining.toString(),
					bufferedDeficit: bufferedRemaining.toString(),
				},
				"UniswapV4 per-position calculation",
			)

			if (credit === 0n) continue

			// Use V4PositionManager.removeCallParameters to generate the calldata
			// This encodes DECREASE_LIQUIDITY + TAKE_PAIR actions internally
			const call = this.buildRemoveLiquidityCall(state, pos, cappedLiq, solver)
			if (!call) continue

			allCalls.push(call)
			state.consume(pos.tokenId, cappedLiq)
			totalCredited += credit
			remaining -= credit

			logger.debug(
				{
					tokenId: pos.tokenId.toString(),
					liquidity: cappedLiq.toString(),
					tokenOut: tokenNeed,
					credited: credit.toString(),
				},
				"UniswapV4 funding planned",
			)
		}

		logger.info(
			{
				callCount: allCalls.length,
				totalCredited: totalCredited.toString(),
			},
			"UniswapV4 planWithdrawalForToken result",
		)

		return { calls: allCalls, credited: totalCredited }
	}

	// =========================================================================
	// Private — liquidity solving
	// =========================================================================

	/**
	 * Binary search to find the minimum liquidity that yields at least `deficit`
	 * of the target token, using the SDK Position's amount calculations.
	 */
	private findLiquidityForDeficit(
		state: UniswapV4LiquidityState,
		pos: HydratedV4Position,
		isToken0: boolean,
		deficit: bigint,
	): bigint {
		const maxLiq = pos.remainingLiquidity
		if (maxLiq === 0n || deficit <= 0n) return 0n

		// Quick check: can the full position cover the deficit?
		const fullPosition = state.buildSdkPosition(pos.tokenId, maxLiq)
		if (!fullPosition) return 0n

		const fullAmount = isToken0
			? BigInt(fullPosition.amount0.quotient.toString())
			: BigInt(fullPosition.amount1.quotient.toString())

		if (fullAmount === 0n) return 0n
		if (fullAmount <= deficit) return maxLiq

		// Binary search for minimum liquidity
		let lo = 1n
		let hi = maxLiq
		let best = maxLiq

		for (let i = 0; i < 48; i++) {
			if (lo > hi) break
			const mid = (lo + hi) / 2n

			const midPosition = state.buildSdkPosition(pos.tokenId, mid)
			if (!midPosition) {
				lo = mid + 1n
				continue
			}

			const midAmount = isToken0
				? BigInt(midPosition.amount0.quotient.toString())
				: BigInt(midPosition.amount1.quotient.toString())

			if (midAmount >= deficit) {
				best = mid
				hi = mid - 1n
			} else {
				lo = mid + 1n
			}
		}

		return best
	}

	// =========================================================================
	// Private — call building
	// =========================================================================

	/**
	 * Builds an ERC-7821 call that wraps the SDK-generated
	 * `removeCallParameters` calldata into a `PositionManager.multicall()`.
	 *
	 * The SDK internally encodes:
	 *   1. DECREASE_LIQUIDITY action (position tokenId, liquidity delta, min amounts)
	 *   2. TAKE_PAIR action (currency0, currency1, recipient)
	 */
	private buildRemoveLiquidityCall(
		state: UniswapV4LiquidityState,
		pos: HydratedV4Position,
		liquidity: bigint,
		solver: HexString,
	): ERC7821Call | null {
		// Build an SDK Position representing what we want to remove
		const sdkPosition = state.buildSdkPosition(pos.tokenId, liquidity)
		if (!sdkPosition) return null

		// Compute the percentage of total position liquidity being removed
		// The SDK expects a Percent representing what fraction of the position to exit
		const totalLiq = pos.liquidity
		if (totalLiq === 0n) return null

		// Calculate percentage with high precision
		// liquidityPercentage = liquidity / totalLiquidity
		const pctNumerator = liquidity * 1_000_000n
		const pctDenominator = totalLiq
		const pctValue = pctNumerator / pctDenominator

		const liquidityPercentage = new Percent(pctValue.toString(), "1000000")

		// Build a full-liquidity Position for the SDK (it computes the removal
		// proportionally based on liquidityPercentage)
		const fullPosition = state.buildSdkPosition(pos.tokenId, totalLiq)
		if (!fullPosition) return null

		const deadline = Math.floor(Date.now() / 1000) + 30 * 60 // 30 min

		const removeOptions: RemoveLiquidityOptions = {
			slippageTolerance: SLIPPAGE_TOLERANCE,
			deadline: deadline.toString(),
			hookData: "0x",
			tokenId: pos.tokenId.toString(),
			liquidityPercentage,
			burnToken: false,
		}

		const { calldata, value } = V4PositionManager.removeCallParameters(fullPosition, removeOptions)

		// Wrap into an ERC-7821 call targeting PositionManager.multicall
		return {
			target: pos.positionManager,
			value: BigInt(value),
			data: encodeFunctionData({
				abi: UNISWAP_V4_POSITION_MANAGER_ABI,
				functionName: "multicall",
				args: [[calldata as HexString]],
			}) as HexString,
		}
	}
}
