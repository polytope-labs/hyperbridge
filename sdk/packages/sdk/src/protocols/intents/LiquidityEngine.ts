import { formatUnits } from "viem"
import { AVAILABLE_LIQUIDITY } from "@/queries"
import type { AvailableLiquiditySnapshot, HexString, IndexerQueryClient } from "@/types"
import { dateStringtoTimestamp } from "@/utils"

const POOL_DEPTH_DECIMALS = 18
const SELL = "SELL"
const BUY = "BUY"

type PoolDirection = typeof SELL | typeof BUY

interface AvailableLiquidityResponse {
	poolChainLiquidities: {
		nodes: PoolChainLiquidityNode[]
	}
	poolRoutes: {
		nodes: PoolRouteNode[]
	}
}

interface PoolChainLiquidityNode {
	depth: string
	bidCount: number
	lastUpdatedAt: string
}

interface PoolRouteNode {
	depth: string
	bidCount: number
}

interface ResolvedPoolQuery {
	poolId: string
	direction: PoolDirection
}

/** Reads pair-centric, route-aware liquidity directly from Nexus. */
export class LiquidityEngine {
	constructor(private readonly queryClient: IndexerQueryClient) {}

	/**
	 * Returns liquidity reachable from one source chain on one destination.
	 *
	 * The caller resolves chain-specific token addresses through chain
	 * configuration; this layer only maps those configured symbols onto the
	 * indexer's canonical pool and route fields.
	 *
	 * Cross-chain queries use only explicitly indexed `PoolRoute` liquidity. The
	 * indexer's unrestricted slice is intentionally excluded because deciding
	 * which sources that legacy policy covers is outside the indexer contract.
	 *
	 * @returns `undefined` only when the indexer has not published a destination
	 * pool sample yet.
	 */
	async getAvailableLiquiditySnapshot(params: {
		sourceChain: string
		destinationChain: string
		tokenInSymbol: string
		tokenOutSymbol: string
		tokenOut: HexString
	}): Promise<AvailableLiquiditySnapshot | undefined> {
		const resolved = resolvePoolQuery(params.tokenInSymbol, params.tokenOutSymbol)
		const variables = {
			poolId: resolved.poolId,
			sourceChain: params.sourceChain,
			destinationChain: params.destinationChain,
			direction: resolved.direction,
		}
		const response = await this.queryClient.request<AvailableLiquidityResponse>(AVAILABLE_LIQUIDITY, variables)
		const chainLiquidity = response.poolChainLiquidities.nodes[0]
		if (!chainLiquidity) return undefined

		const liquidity =
			params.sourceChain === params.destinationChain
				? chainLiquidity
				: (response.poolRoutes.nodes[0] ?? { depth: "0", bidCount: 0 })
		const totalLiquidity = formatUnits(BigInt(liquidity.depth), POOL_DEPTH_DECIMALS)
		return {
			poolId: resolved.poolId,
			direction: resolved.direction,
			sourceChain: params.sourceChain,
			destinationChain: params.destinationChain,
			totalLiquidity,
			providerCount: liquidity.bidCount,
			tokenAddress: params.tokenOut,
			snapshotTime: new Date(dateStringtoTimestamp(chainLiquidity.lastUpdatedAt)),
			liquidityByChain: [
				{
					chain: params.destinationChain,
					tokenAddress: params.tokenOut,
					totalLiquidity,
					providerCount: liquidity.bidCount,
				},
			],
		}
	}
}

function resolvePoolQuery(tokenInSymbol: string, tokenOutSymbol: string): ResolvedPoolQuery {
	const inputIsToken0 = tokenInSymbol.toLowerCase() <= tokenOutSymbol.toLowerCase()
	return {
		poolId: inputIsToken0 ? `${tokenInSymbol}-${tokenOutSymbol}` : `${tokenOutSymbol}-${tokenInSymbol}`,
		direction: inputIsToken0 ? SELL : BUY,
	}
}
