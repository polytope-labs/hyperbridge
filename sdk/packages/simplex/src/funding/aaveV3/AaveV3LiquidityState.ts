import { AAVE_V3_POOL_ABI } from "@/config/abis/AaveV3Pool"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { AaveV3ReserveConfig, HydratedAaveV3Reserve } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { getLogger } from "@/services/Logger"
import type { HexString } from "@hyperbridge/sdk"

const logger = getLogger("aavev3-state")

/**
 * Long-lived Aave V3 liquidity state for one destination chain.
 *
 * Each configured reserve maps an underlying asset (e.g. USDC) to the solver's
 * supplied position. The sourceable amount for a reserve is
 *   min(solver aToken balance, pool's withdrawable reserve)
 * where the withdrawable reserve is the underlying held by the aToken contract.
 * Capping by the latter avoids planning a `withdraw` that would revert because
 * the pool's unborrowed liquidity is insufficient — a revert would roll back
 * the entire ERC-7821 fill batch.
 *
 * Concurrent access is serialised by the planner's per-chain mutex.
 */
export class AaveV3LiquidityState {
	/** Keyed by underlying asset address, lowercased. */
	private reserves = new Map<string, HydratedAaveV3Reserve>()
	private hydrated = false
	/** Per-asset amount reserved for in-flight fills this round, lowercased key. */
	private consumed = new Map<string, bigint>()
	/** Last observed on-chain aToken balance, used to reconcile `consumed`. */
	private lastOnChainBalance = new Map<string, bigint>()

	constructor(
		private readonly chain: string,
		private readonly configs: AaveV3ReserveConfig[],
		private readonly pool: HexString,
		private readonly solver: HexString,
		private readonly clientManager: ChainClientManager,
	) {}

	// =========================================================================
	// Initialisation & refresh
	// =========================================================================

	/**
	 * One-time hydration: resolves each asset's aToken address and decimals,
	 * then loads live balances via {@link refresh}.
	 */
	async hydrate(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		for (const cfg of this.configs) {
			const asset = cfg.asset
			const reserveData = (await client.readContract({
				address: this.pool,
				abi: AAVE_V3_POOL_ABI,
				functionName: "getReserveData",
				args: [asset],
			})) as { aTokenAddress: HexString }

			const aToken = reserveData.aTokenAddress
			if (!aToken || aToken === "0x0000000000000000000000000000000000000000") {
				throw new Error(`AaveV3 reserve ${asset} on ${this.chain} is not listed (no aToken)`)
			}

			const decimals = (await client.readContract({
				address: asset,
				abi: ERC20_ABI,
				functionName: "decimals",
			})) as number

			this.reserves.set(asset.toLowerCase(), {
				asset,
				aToken,
				decimals,
				aTokenBalance: 0n,
				availableReserve: 0n,
				remaining: 0n,
			})
		}

		await this.refresh()
		this.hydrated = true

		for (const r of this.reserves.values()) {
			logger.info(
				{
					chain: this.chain,
					asset: r.asset,
					aToken: r.aToken,
					decimals: r.decimals,
					aTokenBalance: r.aTokenBalance.toString(),
					availableReserve: r.availableReserve.toString(),
				},
				"AaveV3 reserve hydrated",
			)
		}
		logger.info({ chain: this.chain, reserves: this.configs.length }, "AaveV3 liquidity state hydrated")
	}

	/**
	 * Refreshes live balances for every reserve: the solver's aToken balance and
	 * the pool's withdrawable underlying. Reconciles the per-asset `consumed`
	 * counter against any on-chain balance decrease since the last refresh so
	 * executed withdrawals free up their reservation.
	 */
	async refresh(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		for (const r of this.reserves.values()) {
			const key = r.asset.toLowerCase()

			const [aTokenBalance, availableReserve] = await Promise.all([
				client.readContract({
					address: r.aToken,
					abi: ERC20_ABI,
					functionName: "balanceOf",
					args: [this.solver],
				}) as Promise<bigint>,
				client.readContract({
					address: r.asset,
					abi: ERC20_ABI,
					functionName: "balanceOf",
					args: [r.aToken],
				}) as Promise<bigint>,
			])

			// Reconcile consumed against realised on-chain decrease.
			const prevOnChain = this.lastOnChainBalance.get(key) ?? aTokenBalance
			const decrease = prevOnChain > aTokenBalance ? prevOnChain - aTokenBalance : 0n
			const prevConsumed = this.consumed.get(key) ?? 0n
			const newConsumed = prevConsumed > decrease ? prevConsumed - decrease : 0n
			this.consumed.set(key, newConsumed)
			this.lastOnChainBalance.set(key, aTokenBalance)

			r.aTokenBalance = aTokenBalance
			r.availableReserve = availableReserve
			const sourceable = aTokenBalance < availableReserve ? aTokenBalance : availableReserve
			r.remaining = sourceable > newConsumed ? sourceable - newConsumed : 0n

			logger.debug(
				{
					chain: this.chain,
					asset: r.asset,
					aTokenBalance: aTokenBalance.toString(),
					availableReserve: availableReserve.toString(),
					consumed: newConsumed.toString(),
					remaining: r.remaining.toString(),
				},
				"AaveV3 reserve refreshed",
			)
		}
	}

	// =========================================================================
	// Lookups & accounting
	// =========================================================================

	isHydrated(): boolean {
		return this.hydrated
	}

	allReserves(): HydratedAaveV3Reserve[] {
		return Array.from(this.reserves.values())
	}

	/** Reserve whose underlying asset matches `tokenLower`, if configured. */
	reserveForToken(tokenLower: string): HydratedAaveV3Reserve | undefined {
		return this.reserves.get(tokenLower.toLowerCase())
	}

	/** Sourceable amount of `asset` after pending-fill reservations. */
	remaining(asset: string): bigint {
		return this.reserves.get(asset.toLowerCase())?.remaining ?? 0n
	}

	consume(asset: string, amount: bigint): void {
		const key = asset.toLowerCase()
		const r = this.reserves.get(key)
		if (r) {
			r.remaining = r.remaining > amount ? r.remaining - amount : 0n
		}
		this.consumed.set(key, (this.consumed.get(key) ?? 0n) + amount)
	}
}
