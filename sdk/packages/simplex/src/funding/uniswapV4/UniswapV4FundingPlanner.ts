import { UNISWAP_V4_POSITION_MANAGER_ABI } from "@/config/abis/UniswapV4"
import type {
	FundingPlanResult,
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
import { type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { Mutex } from "async-mutex"
import { Decimal } from "decimal.js"
import { Percent } from "@uniswap/sdk-core"
import { V4PositionManager, type Pool as V4Pool } from "@uniswap/v4-sdk"
import type { RemoveLiquidityOptions } from "@uniswap/v4-sdk"
import { encodeFunctionData } from "viem"

const logger = getLogger("uniswapv4-funding")

/** Default slippage tolerance for remove-liquidity operations. */
const DEFAULT_SLIPPAGE_BPS = 50

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
	/** Per-chain mutex serialising planWithdrawalForToken to prevent concurrent state races. */
	private mutexByChain = new Map<string, Mutex>()
	/** Slippage tolerance for remove-liquidity operations, derived from spreadBps. */
	private readonly slippageTolerance: Percent
	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly config: UniswapV4OutputFundingConfig,
		private readonly configService: FillerConfigService,
		spreadBps?: number,
	) {
		const bps = spreadBps ?? DEFAULT_SLIPPAGE_BPS
		this.slippageTolerance = new Percent(bps, 10_000)
	}

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
			this.mutexByChain.set(chain, new Mutex())
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
	// Pricing (FundingVenue)
	// =========================================================================

	async getExoticTokenPrice(chain: string, exoticToken: string): Promise<Decimal | null> {
		const state = this.stateByChain.get(chain)
		if (!state || !state.isHydrated()) return null

		try {
			await state.refresh()
		} catch (err) {
			logger.error({ err, chain }, "Failed to refresh state for price query")
			return null
		}

		const tokenLower = exoticToken.toLowerCase()
		let bestPrice: Decimal | null = null
		let bestLiquidity = 0n

		for (const pos of state.allPositions()) {
			if (pos.currency0.toLowerCase() !== tokenLower && pos.currency1.toLowerCase() !== tokenLower) continue
			const sdkPool = state.getSdkPool(pos.tokenId)
			if (!sdkPool) continue

			const result = this.computeDirectPoolPriceUsd(pos, sdkPool, chain)
			if (result && result.exoticToken.toLowerCase() === tokenLower) {
				const poolLiquidity = state.getPoolLiquidity(pos.tokenId)
				if (poolLiquidity > bestLiquidity) {
					bestPrice = result.priceUsd
					bestLiquidity = poolLiquidity
				}
			}
		}

		if (bestPrice) {
			logger.debug({ chain, token: tokenLower, priceUsd: bestPrice.toString() }, "Exotic token price computed")
		}
		return bestPrice
	}

	/**
	 * Computes the USD price of the non-stable token in a pool.
	 * Returns null if neither currency is USDC/USDT on this chain.
	 */
	private computeDirectPoolPriceUsd(
		pos: HydratedV4Position,
		sdkPool: V4Pool,
		chain: string,
	): { exoticToken: string; priceUsd: Decimal } | null {
		const usdc = this.configService.getUsdcAsset(chain).toLowerCase()
		const usdt = this.configService.getUsdtAsset(chain).toLowerCase()
		const c0 = pos.currency0.toLowerCase()
		const c1 = pos.currency1.toLowerCase()

		if (c0 === usdc || c0 === usdt) {
			// currency0 is stable → exotic is currency1
			// token1Price = "token0 per token1" = USD per exotic
			return {
				exoticToken: c1,
				priceUsd: new Decimal(sdkPool.token1Price.toFixed(18)),
			}
		}

		if (c1 === usdc || c1 === usdt) {
			// currency1 is stable → exotic is currency0
			// token0Price = "token1 per token0" = USD per exotic
			return {
				exoticToken: c0,
				priceUsd: new Decimal(sdkPool.token0Price.toFixed(18)),
			}
		}

		return null
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
	 *
	 * Refreshes on-chain state (liquidity, sqrtPrice) for the destination chain
	 * immediately before planning to ensure calculations use the latest data.
	 */
	async planWithdrawalForToken(
		destChain: string,
		solver: HexString,
		tokenOutLower: string,
		amountNeeded: bigint,
		deadlineTimestamp?: bigint,
	): Promise<FundingPlanResult> {
		const noopResult: FundingPlanResult = { calls: [], credited: 0n }

		logger.debug(
			{
				destChain,
				solver,
				tokenOutLower,
				amountNeeded: amountNeeded.toString(),
			},
			"UniswapV4 planWithdrawalForToken called",
		)

		if (amountNeeded <= 0n) return noopResult

		const state = this.stateByChain.get(destChain)
		if (!state || !state.isHydrated()) {
			logger.debug(
				{ destChain, hasState: !!state, isHydrated: state?.isHydrated() },
				"UniswapV4 no state or not hydrated",
			)
			return noopResult
		}

		const mutex = this.mutexByChain.get(destChain)!
		return mutex.runExclusive(async () => {
			// Refresh on-chain state for this chain right before planning so
			// liquidity and price data are as fresh as possible.
			await state.refresh()

			const tokenNeed = tokenOutLower.toLowerCase()
			const candidates = state
				.positionsForToken(tokenNeed)
				.sort((a, b) => (b.remainingLiquidity > a.remainingLiquidity ? 1 : -1))

			logger.debug(
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

				// Bump the target by the slippage tolerance (10 bps) so the V4
				// withdrawal overshoots the exact amount needed.  This ensures
				// that even in the worst-case slippage scenario the credited
				// tokens still cover the fill requirement, avoiding a wasted
				// revert on the entire ERC-7821 batch.
				const slippageBps = BigInt(this.slippageTolerance.numerator.toString()) * 10_000n / BigInt(this.slippageTolerance.denominator.toString())
				const bufferedRemaining = remaining + (remaining * slippageBps) / 10_000n

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

				logger.debug(
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
				const call = this.buildRemoveLiquidityCall(state, pos, cappedLiq, deadlineTimestamp)
				if (!call) continue

				allCalls.push(call)
				totalCredited += credit
				remaining -= credit
				state.consume(pos.tokenId, cappedLiq)

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

			logger.debug(
				{
					callCount: allCalls.length,
					totalCredited: totalCredited.toString(),
				},
				"UniswapV4 planWithdrawalForToken result",
			)

			return { calls: allCalls, credited: totalCredited }
		}) // mutex.runExclusive
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
		deadlineTimestamp?: bigint,
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

		const deadline = deadlineTimestamp ? Number(deadlineTimestamp) : Math.floor(Date.now() / 1000) + 30 * 60

		const removeOptions: RemoveLiquidityOptions = {
			slippageTolerance: this.slippageTolerance,
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
