import { AAVE_V3_POOL_ABI } from "@/config/abis/AaveV3Pool"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { AaveV3OutputFundingConfig, FundingPlanResult, FundingVenue } from "@/funding/types"
import { AaveV3LiquidityState } from "@/funding/aaveV3/AaveV3LiquidityState"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import { encodeERC7821ExecuteBatch, type ERC7821Call, type HexString } from "@hyperbridge/sdk"
import { Mutex } from "async-mutex"
import type { Decimal } from "decimal.js"
import { encodeFunctionData } from "viem"

const logger = getLogger("aavev3-funding")

/** Default sweep cadence when the config omits `sweepIntervalMs`. */
const DEFAULT_SWEEP_INTERVAL_MS = 5 * 60 * 1000

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
	/** Per-chain mutex serialising sweeps so a slow supply tx can't overlap the next tick. */
	private sweepMutexByChain = new Map<string, Mutex>()
	private solver: HexString | null = null
	private sweepInterval?: NodeJS.Timeout

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly config: AaveV3OutputFundingConfig,
		private readonly configService: FillerConfigService,
	) {}

	/**
	 * Validates raw TOML reserve entries before constructing the planner.
	 * Throws on missing/invalid required fields.
	 */
	static validateConfig(reserves: { chain?: string; asset?: string; threshold?: string; minSweep?: string }[]): void {
		const positiveNumber = (v: string) => /^\d+(\.\d+)?$/.test(v.trim()) && Number(v) > 0
		for (const r of reserves) {
			if (!r.chain?.trim()) {
				throw new Error("Each AaveV3 reserve must have a non-empty 'chain' (e.g. EVM-8453)")
			}
			if (!r.asset?.trim()) {
				throw new Error("Each AaveV3 reserve must include an 'asset' address")
			}
			if (r.threshold !== undefined && !positiveNumber(r.threshold)) {
				throw new Error(`AaveV3 reserve ${r.asset} 'threshold' must be a positive number`)
			}
			if (r.minSweep !== undefined && !positiveNumber(r.minSweep)) {
				throw new Error(`AaveV3 reserve ${r.asset} 'minSweep' must be a positive number`)
			}
		}
	}

	// =========================================================================
	// Lifecycle (FundingVenue)
	// =========================================================================

	async initialise(solver: HexString): Promise<void> {
		// Idempotent: the same shared instance is passed to multiple strategies,
		// each of which calls initialise() during its own startup.
		if (this.solver) return
		this.solver = solver
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
			this.sweepMutexByChain.set(chain, new Mutex())
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

	// =========================================================================
	// Sweeping (LiquiditySweeper) — supply idle wallet balance into Aave
	// =========================================================================

	/**
	 * Starts the periodic sweep timer. Runs one sweep shortly after start, then
	 * every `sweepIntervalMs`. Idempotent — a second call is a no-op.
	 */
	startSweeping(): void {
		if (this.sweepInterval) return
		const intervalMs = this.config.sweepIntervalMs ?? DEFAULT_SWEEP_INTERVAL_MS

		// Initial sweep shortly after start (lets the filler settle first).
		setTimeout(() => {
			this.sweepExcessToPool().catch((err) => logger.error({ err }, "AaveV3 initial sweep failed"))
		}, 30_000)

		this.sweepInterval = setInterval(() => {
			this.sweepExcessToPool().catch((err) => logger.error({ err }, "AaveV3 periodic sweep failed"))
		}, intervalMs)

		logger.info({ intervalMs }, "AaveV3 periodic sweep started")
	}

	stopSweeping(): void {
		if (this.sweepInterval) {
			clearInterval(this.sweepInterval)
			this.sweepInterval = undefined
		}
	}

	/**
	 * Supplies idle wallet balance above each reserve's threshold into Aave for
	 * one chain (or all configured chains). For each reserve, builds an exact
	 * `approve(excess) + supply(excess)` pair and sends them as a single ERC-7821
	 * batch to the solver account — atomic, leaving no residual allowance.
	 */
	async sweepExcessToPool(chain?: string): Promise<void> {
		const chains = chain ? [chain] : Array.from(this.stateByChain.keys())
		await Promise.all(chains.map((c) => this.sweepChain(c)))
	}

	private async sweepChain(chain: string): Promise<void> {
		const state = this.stateByChain.get(chain)
		const solver = this.solver
		if (!state || !state.isHydrated() || !solver) return

		const mutex = this.sweepMutexByChain.get(chain)!
		await mutex.runExclusive(async () => {
			const publicClient = this.clientManager.getPublicClient(chain)
			const pool = this.configService.getAaveV3PoolAddress(chain)
			const calls: ERC7821Call[] = []

			for (const reserve of state.allReserves()) {
				if (reserve.thresholdScaled === null) continue // sweeping disabled for this reserve

				const walletBalance = (await publicClient.readContract({
					abi: ERC20_ABI,
					address: reserve.asset,
					functionName: "balanceOf",
					args: [solver],
				})) as bigint

				const excess = walletBalance > reserve.thresholdScaled ? walletBalance - reserve.thresholdScaled : 0n
				if (excess < reserve.minSweepScaled) continue

				calls.push({
					target: reserve.asset,
					value: 0n,
					data: encodeFunctionData({
						abi: ERC20_ABI,
						functionName: "approve",
						args: [pool, excess],
					}) as HexString,
				})
				calls.push({
					target: pool,
					value: 0n,
					data: encodeFunctionData({
						abi: AAVE_V3_POOL_ABI,
						functionName: "supply",
						args: [reserve.asset, excess, solver, 0],
					}) as HexString,
				})

				logger.info(
					{ chain, asset: reserve.asset, excess: excess.toString() },
					"AaveV3 sweeping excess into pool",
				)
			}

			if (calls.length === 0) return

			const walletClient = this.clientManager.getWalletClient(chain)
			const tx = await walletClient.sendTransaction({
				to: solver,
				data: encodeERC7821ExecuteBatch(calls),
				value: 0n,
				chain: walletClient.chain,
			})
			const receipt = await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1 })
			logger.info({ chain, tx, status: receipt.status, pairs: calls.length / 2 }, "AaveV3 sweep submitted")
		})
	}
}
