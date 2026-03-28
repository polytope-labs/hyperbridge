import type { HexString } from "@hyperbridge/sdk"
import { type ERC7821Call, encodeERC7821ExecuteBatch } from "@hyperbridge/sdk"
import { encodeFunctionData, maxUint256 } from "viem"
import { Mutex } from "async-mutex"
import type { Decimal } from "decimal.js"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { FundingPlanResult, FundingVenue, AerodromeOutputFundingConfig, HydratedPool } from "@/funding/types"
import { AerodromeLiquidityState } from "@/funding/aerodrome/AerodromeLiquidityState"
import { AERODROME_GAUGE_ABI, AERODROME_ROUTER_ABI } from "@/config/abis/Aerodrome"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { getLogger } from "@/services/Logger"

const logger = getLogger("aerodrome-funding")

/** Default slippage for `amount0Min` / `amount1Min` (50 bps). */
const DEFAULT_MIN_AMOUNT_OUT_BPS = 9950
const MAX_BS_ITER = 48

// ============================================================================
// Helpers
// ============================================================================

function sortTokens(t0: HexString, t1: HexString): [HexString, HexString] {
	return t0.toLowerCase() < t1.toLowerCase() ? [t0, t1] : [t1, t0]
}

/**
 * Map router (tokenA, tokenB) amounts → pair (token0, token1) order.
 * `tokenA` is whichever address was passed first to the router call.
 */
function mapRouterAmountsToToken01(
	token0: HexString,
	tokenA: HexString,
	amountA: bigint,
	amountB: bigint,
): { amount0: bigint; amount1: bigint } {
	if (tokenA.toLowerCase() === token0.toLowerCase()) {
		return { amount0: amountA, amount1: amountB }
	}
	return { amount0: amountB, amount1: amountA }
}

/**
 * Reverse of above: token0/token1 mins → router's tokenA/tokenB order.
 */
function mapToken01MinsToRouter(
	token0: HexString,
	tokenA: HexString,
	amount0Min: bigint,
	amount1Min: bigint,
): { amountAMin: bigint; amountBMin: bigint } {
	if (tokenA.toLowerCase() === token0.toLowerCase()) {
		return { amountAMin: amount0Min, amountBMin: amount1Min }
	}
	return { amountAMin: amount1Min, amountBMin: amount0Min }
}

/**
 * Closed-form LP needed for a volatile pool.
 * `LP = ceil(deficit * totalSupply / reserveOut)`
 */
function liquidityForVolatileDeficit(deficit: bigint, reserveOut: bigint, totalSupply: bigint): bigint {
	if (deficit <= 0n || reserveOut === 0n || totalSupply === 0n) return 0n
	return (deficit * totalSupply + reserveOut - 1n) / reserveOut
}

// ============================================================================
// Planner
// ============================================================================

export class AerodromeFundingPlanner implements FundingVenue {
	name = "Aerodrome"
	/** Long-lived state per chain, keyed by chain identifier. */
	private stateByChain = new Map<string, AerodromeLiquidityState>()
	/** Per-chain mutex serialising planWithdrawalForToken to prevent concurrent state races. */
	private mutexByChain = new Map<string, Mutex>()

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly config: AerodromeOutputFundingConfig,
		private readonly configService: FillerConfigService,
		spreadBps?: number,
	) {
		this._minAmountOutBps = spreadBps !== undefined ? 10000 - spreadBps : DEFAULT_MIN_AMOUNT_OUT_BPS
	}

	private readonly _minAmountOutBps: number

	/**
	 * Validates raw TOML pool entries before constructing the planner.
	 * Throws on missing/invalid required fields. Address availability
	 * is checked later in `initialise()` when configService is available.
	 */
	static validateConfig(pools: { chain?: string; pair?: string; gauge?: string }[]): void {
		for (const pool of pools) {
			if (!pool.chain?.trim()) {
				throw new Error("Each Aerodrome vault pool must have a non-empty 'chain' (e.g. EVM-8453)")
			}
			if (!pool.pair) {
				throw new Error("Each Aerodrome pool must include a 'pair' address")
			}
		}
	}

	get minAmountOutBps(): number {
		return this._minAmountOutBps
	}

	// =========================================================================
	// Lifecycle
	// =========================================================================

	/**
	 * Call once at startup.  Hydrates every configured chain's pools
	 * (reads token0/token1/stable, validates gauge, fetches initial reserves
	 * and LP balances).  Replaces the old `validateAerodromePoolsOnChain`.
	 */
	async initialise(solver: HexString): Promise<void> {
		const approvalsByChain = new Map<string, ERC7821Call[]>()
		for (const [chain, pools] of Object.entries(this.config.poolsByChain)) {
			const router = this.configService.getAerodromeRouterAddress(chain)
			const z = router.toLowerCase()
			if (!z || z === "0x" || z === "0x0000000000000000000000000000000000000000") {
				throw new Error(`Aerodrome router not configured for chain ${chain}`)
			}
			const state = new AerodromeLiquidityState(chain, pools, router, solver, this.clientManager)
			await state.hydrate()
			this.stateByChain.set(chain, state)
			this.mutexByChain.set(chain, new Mutex())

			const client = this.clientManager.getPublicClient(chain)
			const calls: ERC7821Call[] = []
			for (const pool of state.allPools()) {
				const allowance = (await client.readContract({
					address: pool.pair,
					abi: ERC20_ABI,
					functionName: "allowance",
					args: [solver, pool.router],
				})) as bigint
				if (allowance < maxUint256 / 2n) {
					calls.push({
						target: pool.pair,
						value: 0n,
						data: encodeFunctionData({
							abi: ERC20_ABI,
							functionName: "approve",
							args: [pool.router, maxUint256],
						}) as HexString,
					})
				}
			}
			if (calls.length > 0) {
				approvalsByChain.set(chain, calls)
			}
		}
		for (const [chain, calls] of approvalsByChain) {
			const walletClient = this.clientManager.getWalletClient(chain)
			const callData = encodeERC7821ExecuteBatch(calls)
			await walletClient.sendTransaction({
				to: solver,
				data: callData,
				value: 0n,
				chain: walletClient.chain,
			})
			logger.info({ chain, approvalCount: calls.length }, "Batched LP token approvals via ERC-7821")
		}
	}

	/**
	 * Refresh live data (reserves, LP balances) for one or all chains.
	 * Called automatically at the start of planWithdrawalForToken for the
	 * relevant chain.
	 */
	async refresh(chain?: string): Promise<void> {
		if (chain) {
			const state = this.stateByChain.get(chain)
			if (state) await state.refresh()
		} else {
			await Promise.all(Array.from(this.stateByChain.values()).map((s) => s.refresh()))
		}
	}

	/** Retrieve the liquidity state for a chain.  Returns undefined if not configured. */
	getState(chain: string): AerodromeLiquidityState | undefined {
		return this.stateByChain.get(chain)
	}

	async getExoticTokenPrice(_chain: string, _exoticToken: string): Promise<Decimal | null> {
		return null
	}

	// =========================================================================
	// Planning
	// =========================================================================

	/**
	 * Produces ERC-7821 calls to withdraw LP and remove liquidity so that at
	 * least `amountNeeded` of `tokenOut` is credited to the solver.
	 *
	 * Refreshes on-chain state (reserves, LP balances) for the destination
	 * chain immediately before planning to ensure calculations use the latest
	 * data.  Only stable-pool binary search still needs additional RPC calls
	 * (quoteRemoveLiquidity).
	 */
	async planWithdrawalForToken(
		destChain: string,
		solver: HexString,
		tokenOutLower: string,
		amountNeeded: bigint,
		deadlineTimestamp?: bigint,
	): Promise<FundingPlanResult> {
		const noopResult: FundingPlanResult = { calls: [], credited: 0n }

		if (amountNeeded <= 0n) return noopResult

		const state = this.stateByChain.get(destChain)
		if (!state || !state.isHydrated()) return noopResult

		const mutex = this.mutexByChain.get(destChain)!
		return mutex.runExclusive(async () => {
			// Refresh on-chain state for this chain right before planning so
			// reserves and LP balances are as fresh as possible.
			await state.refresh()

			const tokenNeed = tokenOutLower.toLowerCase()
			const candidatePools = state.poolsForToken(tokenNeed)

			const slippageBps = 10000n - BigInt(this.minAmountOutBps)
			const bufferedAmount = amountNeeded + (amountNeeded * slippageBps) / 10000n

			for (const pool of candidatePools) {
				const lpAvail = state.remaining(pool.pair)
				if (lpAvail === 0n) continue

				const [tokenA, tokenB] = sortTokens(pool.token0, pool.token1)

				// --- solve LP amount ---
				const liquidity =
					this.solveLiquidityForDeficit(pool, tokenNeed, bufferedAmount, lpAvail) ??
					(await this.solveLiquidityForDeficitStable(
						destChain,
						pool,
						tokenA,
						tokenB,
						tokenNeed,
						bufferedAmount,
						lpAvail,
					))
				if (liquidity <= 0n) continue

				const cappedL = liquidity > lpAvail ? lpAvail : liquidity

				// --- quote expected output ---
				const expected = await this.quoteExpectedAmounts(destChain, pool, tokenA, tokenB, cappedL)
				if (!expected) continue

				const { amount0, amount1 } = mapRouterAmountsToToken01(
					pool.token0,
					tokenA,
					expected.amountA,
					expected.amountB,
				)
				const credit = tokenNeed === pool.token0.toLowerCase() ? amount0 : amount1
				if (credit === 0n) continue

				// --- slippage ---
				// By the time the tx lands, reserves can change (other swaps, liquidity moves, ordering in the block).
				const bps = BigInt(this.minAmountOutBps)
				const amount0Min = (amount0 * bps) / 10000n
				const amount1Min = (amount1 * bps) / 10000n
				const { amountAMin, amountBMin } = mapToken01MinsToRouter(pool.token0, tokenA, amount0Min, amount1Min)

				// --- build ERC-7821 calls ---
				const deadline = deadlineTimestamp ?? BigInt(Math.floor(Date.now() / 1000) + 30 * 60)
				const calls: ERC7821Call[] = []

				// 1. Gauge withdraw (if needed)
				if (pool.gauge) {
					// Only withdraw the shortfall beyond what's already in the wallet.
					const shortfall = cappedL > pool.walletLp ? cappedL - pool.walletLp : 0n
					const withdrawAmt = shortfall > pool.gaugeLp ? pool.gaugeLp : shortfall
					if (withdrawAmt > 0n) {
						calls.push({
							target: pool.gauge,
							value: 0n,
							data: encodeFunctionData({
								abi: AERODROME_GAUGE_ABI,
								functionName: "withdraw",
								args: [withdrawAmt],
							}) as HexString,
						})
					}
				}

				// 2. Remove liquidity
				calls.push({
					target: pool.router,
					value: 0n,
					data: encodeFunctionData({
						abi: AERODROME_ROUTER_ABI,
						functionName: "removeLiquidity",
						args: [tokenA, tokenB, pool.stable, cappedL, amountAMin, amountBMin, solver, deadline],
					}) as HexString,
				})

				logger.debug(
					{
						pair: pool.pair,
						liquidity: cappedL.toString(),
						tokenOut: tokenNeed,
						credited: credit.toString(),
					},
					"Aerodrome funding planned",
				)

				state.consume(pool.pair, cappedL)
				return { calls, credited: credit }
			}

			return noopResult
		}) // mutex.runExclusive
	}

	// =========================================================================
	// Private — LP solving
	// =========================================================================

	/**
	 * Volatile pool: closed-form solve from cached reserves + totalSupply.
	 * Returns `null` for stable pools (caller falls through to binary search).
	 */
	private solveLiquidityForDeficit(
		pool: HydratedPool,
		tokenNeedLower: string,
		deficit: bigint,
		lpMax: bigint,
	): bigint | null {
		if (pool.stable) return null
		if (lpMax === 0n || deficit <= 0n) return 0n

		const reserveOut = tokenNeedLower === pool.token0.toLowerCase() ? pool.reserve0 : pool.reserve1

		const L = liquidityForVolatileDeficit(deficit, reserveOut, pool.totalSupply)
		return L > lpMax ? lpMax : L
	}

	/**
	 * Stable pool: binary search over `quoteRemoveLiquidity`.
	 * This is the only path that still needs RPC calls during planning.
	 */
	private async solveLiquidityForDeficitStable(
		chain: string,
		pool: HydratedPool,
		tokenA: HexString,
		tokenB: HexString,
		tokenNeedLower: string,
		deficit: bigint,
		lpMax: bigint,
	): Promise<bigint> {
		if (lpMax === 0n || deficit <= 0n) return 0n

		let lo = 0n
		let hi = lpMax
		let best = 0n

		for (let i = 0; i < MAX_BS_ITER; i++) {
			if (lo > hi) break
			const mid = (lo + hi) / 2n
			if (mid === 0n) {
				lo = 1n
				continue
			}

			const q = await this.quoteExpectedAmounts(chain, pool, tokenA, tokenB, mid)
			if (!q) {
				hi = mid - 1n
				continue
			}

			const { amount0, amount1 } = mapRouterAmountsToToken01(pool.token0, tokenA, q.amountA, q.amountB)
			const out = tokenNeedLower === pool.token0.toLowerCase() ? amount0 : amount1

			if (out >= deficit) {
				best = mid
				hi = mid - 1n
			} else {
				lo = mid + 1n
			}
		}

		return best
	}

	// =========================================================================
	// Private — Router quote
	// =========================================================================

	private async quoteExpectedAmounts(
		chain: string,
		pool: HydratedPool,
		tokenA: HexString,
		tokenB: HexString,
		liquidity: bigint,
	): Promise<{ amountA: bigint; amountB: bigint } | null> {
		try {
			const client = this.clientManager.getPublicClient(chain)
			const [amountA, amountB] = await client.readContract({
				address: pool.router,
				abi: AERODROME_ROUTER_ABI,
				functionName: "quoteRemoveLiquidity",
				args: [tokenA, tokenB, pool.stable, liquidity],
			})
			return { amountA, amountB }
		} catch {
			return null
		}
	}
}
