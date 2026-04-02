import type { HexString } from "@hyperbridge/sdk"
import type { PublicClient } from "viem"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { AerodromePoolConfig, HydratedPool } from "@/funding/types"
import { AERODROME_PAIR_ABI, AERODROME_GAUGE_ABI } from "@/config/abis/Aerodrome"
import { getLogger } from "@/services/Logger"

const logger = getLogger("aerodrome-state")

/**
 * Long-lived liquidity state for Aerodrome pools on a single destination chain.
 *
 * Hydrates pool metadata (token0, token1, stable) once, then periodically
 * refreshes live data (reserves, totalSupply, LP balances).  The planner
 * reads from this cache instead of hitting the chain per-order.
 *
 * Concurrent access is serialised by the planner's per-chain mutex.
 */
export class AerodromeLiquidityState {
	/** Keyed by `pair` address (lower-cased). */
	private pools = new Map<string, HydratedPool>()
	private hydrated = false
	private consumed = new Map<string, bigint>()
	private lastOnChainLp = new Map<string, bigint>()

	constructor(
		private readonly chain: string,
		private readonly configs: AerodromePoolConfig[],
		private readonly router: HexString,
		private readonly solver: HexString,
		private readonly clientManager: ChainClientManager,
	) {}

	// =========================================================================
	// Initialisation & refresh
	// =========================================================================

	/**
	 * One-time hydration: reads immutable pool metadata (token0, token1, stable)
	 * and then calls `refresh()` for live data.  Throws on mismatches (acts as
	 * the old `validatePools` step).
	 */
	async hydrate(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		for (const cfg of this.configs) {
			const key = cfg.pair.toLowerCase()

			const [token0, token1, stable] = await Promise.all([
				client.readContract({
					address: cfg.pair,
					abi: AERODROME_PAIR_ABI,
					functionName: "token0",
				}) as Promise<HexString>,
				client.readContract({
					address: cfg.pair,
					abi: AERODROME_PAIR_ABI,
					functionName: "token1",
				}) as Promise<HexString>,
				client.readContract({
					address: cfg.pair,
					abi: AERODROME_PAIR_ABI,
					functionName: "stable",
				}) as Promise<boolean>,
			])

			// Validate gauge staking token matches the pair (smoke-check).
			if (cfg.gauge) {
				const stakingToken = (await client.readContract({
					address: cfg.gauge,
					abi: AERODROME_GAUGE_ABI,
					functionName: "stakingToken",
				})) as HexString

				if (stakingToken.toLowerCase() !== cfg.pair.toLowerCase()) {
					throw new Error(
						`Aerodrome gauge ${cfg.gauge}: stakingToken ${stakingToken} does not match pair ${cfg.pair}`,
					)
				}
			}

			this.pools.set(key, {
				pair: cfg.pair,
				token0,
				token1,
				stable,
				router: this.router,
				gauge: cfg.gauge,
				// Zeroed until refresh().
				reserve0: 0n,
				reserve1: 0n,
				totalSupply: 0n,
				remainingLp: 0n,
				walletLp: 0n,
				gaugeLp: 0n,
			})
		}

		await this.refresh()
		this.hydrated = true

		logger.info({ chain: this.chain, pools: this.configs.length }, "Aerodrome liquidity state hydrated")
	}

	/**
	 * Refreshes live data: reserves, totalSupply, and LP balances.
	 * Called on-demand by the planner before each withdrawal plan.
	 */
	async refresh(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)

		const refreshPromises = Array.from(this.pools.values()).map((pool) => this.refreshOnePool(client, pool))
		await Promise.all(refreshPromises)
	}

	private async refreshOnePool(client: PublicClient, pool: HydratedPool): Promise<void> {
		const [reserves, totalSupply, walletLp, gaugeLp] = await Promise.all([
			client.readContract({
				address: pool.pair,
				abi: AERODROME_PAIR_ABI,
				functionName: "getReserves",
			}) as Promise<[bigint, bigint, bigint]>,
			client.readContract({
				address: pool.pair,
				abi: AERODROME_PAIR_ABI,
				functionName: "totalSupply",
			}) as Promise<bigint>,
			client.readContract({
				address: pool.pair,
				abi: AERODROME_PAIR_ABI,
				functionName: "balanceOf",
				args: [this.solver],
			}) as Promise<bigint>,
			pool.gauge
				? (client.readContract({
						address: pool.gauge,
						abi: AERODROME_GAUGE_ABI,
						functionName: "balanceOf",
						args: [this.solver],
					}) as Promise<bigint>)
				: Promise.resolve(0n),
		])

		pool.reserve0 = reserves[0]
		pool.reserve1 = reserves[1]
		pool.totalSupply = totalSupply

		pool.walletLp = walletLp
		pool.gaugeLp = gaugeLp

		const key = pool.pair.toLowerCase()
		const onChain = walletLp + gaugeLp
		const prevOnChain = this.lastOnChainLp.get(key) ?? onChain
		const decrease = prevOnChain > onChain ? prevOnChain - onChain : 0n
		const prevConsumed = this.consumed.get(key) ?? 0n
		const newConsumed = prevConsumed > decrease ? prevConsumed - decrease : 0n
		this.consumed.set(key, newConsumed)
		this.lastOnChainLp.set(key, onChain)
		pool.remainingLp = onChain > newConsumed ? onChain - newConsumed : 0n
	}

	// =========================================================================
	// Pool lookups
	// =========================================================================

	isHydrated(): boolean {
		return this.hydrated
	}

	/** All hydrated pools for this chain. */
	allPools(): HydratedPool[] {
		return Array.from(this.pools.values())
	}

	/** Pools that contain `tokenLower` as either token0 or token1. */
	poolsForToken(tokenLower: string): HydratedPool[] {
		const t = tokenLower.toLowerCase()
		return this.allPools().filter((p) => p.token0.toLowerCase() === t || p.token1.toLowerCase() === t)
	}

	getPool(pair: HexString): HydratedPool | undefined {
		return this.pools.get(pair.toLowerCase())
	}

	/** Remaining LP available for a given pair. */
	remaining(pair: HexString): bigint {
		return this.pools.get(pair.toLowerCase())?.remainingLp ?? 0n
	}

	consume(pair: HexString, amount: bigint): void {
		const key = pair.toLowerCase()
		const pool = this.pools.get(key)
		if (pool) {
			pool.remainingLp = pool.remainingLp > amount ? pool.remainingLp - amount : 0n
		}
		this.consumed.set(key, (this.consumed.get(key) ?? 0n) + amount)
	}
}
