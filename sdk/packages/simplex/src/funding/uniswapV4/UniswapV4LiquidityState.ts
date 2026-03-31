import { UNISWAP_V4_POSITION_MANAGER_ABI, UNISWAP_V4_STATE_VIEW_ABI, decodeTickLower, decodeTickUpper } from "@/config/abis/UniswapV4"
import type { HydratedV4Position, UniswapV4PositionInit } from "@/funding/types"
import { chainIdFromIdentifier, currencyFromHydratedDecimals } from "@/funding/uniswapV4/v4PoolCurrency"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { getLogger } from "@/services/Logger"
import type { HexString } from "@hyperbridge/sdk"
import { Pool as V4Pool, Position as V4Position } from "@uniswap/v4-sdk"

const logger = getLogger("uniswapv4-state")

/**
 * Long-lived liquidity state for Uniswap V4 positions on a single destination chain.
 *
 * Uses the official `@uniswap/v4-sdk` Pool and Position classes for on-chain
 * hydration and amount calculations.  Pool state is fetched via the StateView
 * contract (getSlot0, getLiquidity) rather than raw extsload.
 *
 * Concurrent access is serialised by the planner's per-chain mutex.
 */
export class UniswapV4LiquidityState {
	/** Keyed by tokenId.toString(). */
	private positions = new Map<string, HydratedV4Position>()
	/** SDK Pool objects, keyed by poolId hex string. */
	private sdkPools = new Map<string, V4Pool>()
	/** Maps tokenId → poolId for quick lookup. */
	private tokenIdToPoolId = new Map<string, string>()
	private hydrated = false
	private consumed = new Map<string, bigint>()
	private lastOnChainLiquidity = new Map<string, bigint>()

	constructor(
		private readonly chain: string,
		private readonly configs: UniswapV4PositionInit[],
		private readonly positionManager: HexString,
		private readonly poolManager: HexString,
		private readonly stateView: HexString,
		private readonly solver: HexString,
		private readonly clientManager: ChainClientManager,
	) {}

	// =========================================================================
	// Initialisation & refresh
	// =========================================================================

	/**
	 * One-time hydration: reads position metadata (poolKey, tick range) and
	 * initial liquidity + slot0 from on-chain via StateView.  Constructs SDK
	 * Pool and Position objects for downstream math.
	 */
	async hydrate(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)
		const chainId = chainIdFromIdentifier(this.chain)

		for (const cfg of this.configs) {
			const key = cfg.tokenId.toString()

			// Verify ownership
			const owner = (await client.readContract({
				address: this.positionManager,
				abi: UNISWAP_V4_POSITION_MANAGER_ABI,
				functionName: "ownerOf",
				args: [cfg.tokenId],
			})) as HexString

			if (owner.toLowerCase() !== this.solver.toLowerCase()) {
				throw new Error(
					`UniswapV4 position ${cfg.tokenId}: owner ${owner} does not match solver ${this.solver}`,
				)
			}

			const poolKey = cfg.poolKey
			const positionInfo = cfg.positionInfo

			// Decode packed position info for tick bounds
			// positionInfo layout: tickLower (int24, bits 232..255), tickUpper (int24, bits 208..231)
			const tickLower = decodeTickLower(positionInfo)
			const tickUpper = decodeTickUpper(positionInfo)

			// Compute the SDK PoolId to group positions sharing a pool
			const currency0Sdk = currencyFromHydratedDecimals(chainId, poolKey.currency0, cfg.decimals0)
			const currency1Sdk = currencyFromHydratedDecimals(chainId, poolKey.currency1, cfg.decimals1)
			const poolId = V4Pool.getPoolId(currency0Sdk, currency1Sdk, poolKey.fee, poolKey.tickSpacing, poolKey.hooks)

			this.tokenIdToPoolId.set(key, poolId)

			this.positions.set(key, {
				tokenId: cfg.tokenId,
				positionManager: this.positionManager,
				poolManager: this.poolManager,
				currency0: poolKey.currency0,
				currency1: poolKey.currency1,
				decimals0: cfg.decimals0,
				decimals1: cfg.decimals1,
				fee: poolKey.fee,
				tickSpacing: poolKey.tickSpacing,
				hooks: poolKey.hooks,
				tickLower,
				tickUpper,
				// Zeroed until refresh().
				liquidity: 0n,
				remainingLiquidity: 0n,
				sqrtPriceX96: 0n,
				currentTick: 0,
			})
		}

		await this.refresh()
		this.hydrated = true

		for (const pos of this.positions.values()) {
			logger.info(
				{
					chain: this.chain,
					tokenId: pos.tokenId.toString(),
					currency0: pos.currency0,
					currency1: pos.currency1,
					fee: pos.fee,
					tickSpacing: pos.tickSpacing,
					tickLower: pos.tickLower,
					tickUpper: pos.tickUpper,
					liquidity: pos.liquidity.toString(),
					sqrtPriceX96: pos.sqrtPriceX96.toString(),
					currentTick: pos.currentTick,
				},
				"UniswapV4 position hydrated",
			)
		}
		logger.info({ chain: this.chain, positions: this.configs.length }, "UniswapV4 liquidity state hydrated")
	}

	/**
	 * Refreshes live data: position liquidity and pool slot0 (sqrtPrice, tick)
	 * via the StateView contract.  Reconstructs SDK Pool objects so the
	 * FundingPlanner can use the Position class for amount calculations.
	 */
	async refresh(): Promise<void> {
		const client = this.clientManager.getPublicClient(this.chain)
		const chainId = chainIdFromIdentifier(this.chain)

		// Group positions by poolId to avoid duplicate pool state fetches
		const poolIds = new Set(this.tokenIdToPoolId.values())

		// Fetch slot0 + liquidity for each unique pool via StateView
		const poolStateMap = new Map<string, { sqrtPriceX96: bigint; tick: number; poolLiquidity: bigint }>()

		for (const poolId of poolIds) {
			const [slot0Result, poolLiquidity] = await Promise.all([
				client.readContract({
					address: this.stateView,
					abi: UNISWAP_V4_STATE_VIEW_ABI,
					functionName: "getSlot0",
					args: [poolId as HexString],
				}) as Promise<[bigint, number, number, number]>,
				client.readContract({
					address: this.stateView,
					abi: UNISWAP_V4_STATE_VIEW_ABI,
					functionName: "getLiquidity",
					args: [poolId as HexString],
				}) as Promise<bigint>,
			])

			poolStateMap.set(poolId, {
				sqrtPriceX96: slot0Result[0],
				tick: slot0Result[1],
				poolLiquidity,
			})
		}

		// Refresh per-position liquidity and rebuild SDK Pool objects
		for (const pos of this.positions.values()) {
			const key = pos.tokenId.toString()
			const poolId = this.tokenIdToPoolId.get(key)
			const poolState = poolId ? poolStateMap.get(poolId) : undefined
			if (!poolId || !poolState) {
				throw new Error(
					`UniswapV4 refresh: missing pool state for tokenId ${key} (poolId=${poolId ?? "undefined"})`,
				)
			}

			// Read current position liquidity
			const liquidity = (await client.readContract({
				address: pos.positionManager,
				abi: UNISWAP_V4_POSITION_MANAGER_ABI,
				functionName: "getPositionLiquidity",
				args: [pos.tokenId],
			})) as bigint

			pos.liquidity = liquidity
			const prevOnChain = this.lastOnChainLiquidity.get(key) ?? liquidity
			const decrease = prevOnChain > liquidity ? prevOnChain - liquidity : 0n
			const prevConsumed = this.consumed.get(key) ?? 0n
			const newConsumed = prevConsumed > decrease ? prevConsumed - decrease : 0n
			this.consumed.set(key, newConsumed)
			this.lastOnChainLiquidity.set(key, liquidity)
			pos.remainingLiquidity = liquidity > newConsumed ? liquidity - newConsumed : 0n
			pos.sqrtPriceX96 = poolState.sqrtPriceX96
			pos.currentTick = poolState.tick

			// Build SDK Pool (without tick data provider — we only need amount calcs)
			const currency0 = currencyFromHydratedDecimals(chainId, pos.currency0, pos.decimals0)
			const currency1 = currencyFromHydratedDecimals(chainId, pos.currency1, pos.decimals1)

			const sdkPool = new V4Pool(
				currency0,
				currency1,
				pos.fee,
				pos.tickSpacing,
				pos.hooks,
				poolState.sqrtPriceX96.toString(),
				poolState.poolLiquidity.toString(),
				poolState.tick,
			)

			this.sdkPools.set(poolId, sdkPool)

			logger.debug(
				{
					tokenId: pos.tokenId.toString(),
					liquidity: liquidity.toString(),
					remainingLiquidity: pos.remainingLiquidity.toString(),
					sqrtPriceX96: poolState.sqrtPriceX96.toString(),
					currentTick: poolState.tick,
				},
				"UniswapV4 position refreshed",
			)
		}
	}

	// =========================================================================
	// Position & Pool lookups
	// =========================================================================

	isHydrated(): boolean {
		return this.hydrated
	}

	/** All hydrated positions for this chain. */
	allPositions(): HydratedV4Position[] {
		return Array.from(this.positions.values())
	}

	/** Positions that contain `tokenLower` as either currency0 or currency1. */
	positionsForToken(tokenLower: string): HydratedV4Position[] {
		const t = tokenLower.toLowerCase()
		return this.allPositions().filter((p) => p.currency0.toLowerCase() === t || p.currency1.toLowerCase() === t)
	}

	getPosition(tokenId: bigint): HydratedV4Position | undefined {
		return this.positions.get(tokenId.toString())
	}

	/**
	 * Returns the SDK Pool object for a given position.
	 * Used by the FundingPlanner to construct Position objects for amount calcs.
	 */
	getSdkPool(tokenId: bigint): V4Pool | undefined {
		const poolId = this.tokenIdToPoolId.get(tokenId.toString())
		return poolId ? this.sdkPools.get(poolId) : undefined
	}

	/**
	 * Build an SDK Position instance for a given tokenId with the specified
	 * liquidity amount.  Uses the cached SDK Pool for price/tick data.
	 */
	buildSdkPosition(tokenId: bigint, liquidity: bigint): V4Position | undefined {
		const pos = this.positions.get(tokenId.toString())
		if (!pos) return undefined
		const pool = this.getSdkPool(tokenId)
		if (!pool) return undefined

		return new V4Position({
			pool,
			tickLower: pos.tickLower,
			tickUpper: pos.tickUpper,
			liquidity: liquidity.toString(),
		})
	}

	/** Remaining liquidity available for a given position. */
	remaining(tokenId: bigint): bigint {
		return this.positions.get(tokenId.toString())?.remainingLiquidity ?? 0n
	}

	consume(tokenId: bigint, amount: bigint): void {
		const key = tokenId.toString()
		const pos = this.positions.get(key)
		if (pos) {
			pos.remainingLiquidity = pos.remainingLiquidity > amount ? pos.remainingLiquidity - amount : 0n
		}
		this.consumed.set(key, (this.consumed.get(key) ?? 0n) + amount)
	}

	/** Returns the SDK Pool's total active liquidity for sorting by depth. */
	getPoolLiquidity(tokenId: bigint): bigint {
		const poolId = this.tokenIdToPoolId.get(tokenId.toString())
		const pool = poolId ? this.sdkPools.get(poolId) : undefined
		return pool ? BigInt(pool.liquidity.toString()) : 0n
	}
}

