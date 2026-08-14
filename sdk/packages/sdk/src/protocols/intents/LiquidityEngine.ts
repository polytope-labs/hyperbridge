import { formatUnits } from "viem"
import type { Chains, ConfiguredAssetSymbol } from "@/configs/chain"
import { AVAILABLE_LIQUIDITY, BUY_AND_SELL_RATES } from "@/queries"
import type { AvailableLiquidity, BuyAndSellRates, HexString, IndexerQueryClient, LiquiditySlice } from "@/types"
import { dateStringtoTimestamp, normalizeEvmAddress } from "@/utils"
import { resolveLiquidityPool } from "./liquidity-pool"

const POOL_DEPTH_DECIMALS = 18
const SELL = "SELL"
const BUY = "BUY"

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
	unrestrictedDepth: string
	unrestrictedBidCount: number
	lastUpdatedAt: string
}

interface PoolRouteNode {
	depth: string
	bidCount: number
	lastUpdatedAt: string
}

interface BuyAndSellRatesResponse {
	liquidityPools: {
		nodes: Array<{
			sellRate: string | null
			buyRate: string | null
			lastUpdatedAt: string
		}>
	}
}

export interface LiquidityAsset {
	chain: Chains
	symbol: ConfiguredAssetSymbol
	address: HexString
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
	 * Destination, unrestricted, and explicit-route capacity are returned as
	 * separate values so callers can apply their own source-chain policy.
	 *
	 * @returns `undefined` only when the indexer has not published a destination
	 * pool sample yet.
	 */
	async getAvailableLiquidity(params: {
		source: LiquidityAsset
		destination: LiquidityAsset
	}): Promise<AvailableLiquidity | undefined> {
		const pool = resolveLiquidityPool(params.source.symbol, params.destination.symbol)
		const direction = params.source.symbol.toLowerCase() === pool.token0Symbol.toLowerCase() ? SELL : BUY
		const variables = {
			poolId: pool.poolId,
			sourceChain: params.source.chain,
			destinationChain: params.destination.chain,
			direction,
		}
		const response = await this.queryClient.request<AvailableLiquidityResponse>(AVAILABLE_LIQUIDITY, variables)
		if (!response?.poolChainLiquidities?.nodes || !response?.poolRoutes?.nodes) {
			throw new InvalidLiquidityIndexerResponseError("liquidity connections are missing")
		}
		const chainLiquidity = response.poolChainLiquidities.nodes[0]
		if (!chainLiquidity) return undefined

		const route = response.poolRoutes.nodes[0]
		return {
			sourceChain: params.source.chain,
			destinationChain: params.destination.chain,
			tokenAddress: normalizeEvmAddress(params.destination.address, "destination token"),
			updatedAt: readIndexerDate(chainLiquidity.lastUpdatedAt, "destination lastUpdatedAt"),
			destination: readLiquiditySlice(chainLiquidity.depth, chainLiquidity.bidCount, "destination"),
			unrestricted: readLiquiditySlice(
				chainLiquidity.unrestrictedDepth,
				chainLiquidity.unrestrictedBidCount,
				"unrestricted",
			),
			explicitRoute: route
				? {
						...readLiquiditySlice(route.depth, route.bidCount, "explicit route"),
						updatedAt: readIndexerDate(route.lastUpdatedAt, "route lastUpdatedAt"),
					}
				: null,
		}
	}

	/** Returns the indexer's merged buy and sell rates for a configured symbol pair. */
	async getBuyAndSellRates(params: {
		sourceChain: Chains
		destinationChain: Chains
		tokenInSymbol: ConfiguredAssetSymbol
		tokenOutSymbol: ConfiguredAssetSymbol
	}): Promise<BuyAndSellRates | undefined> {
		const pool = resolveLiquidityPool(params.tokenInSymbol, params.tokenOutSymbol)
		const response = await this.queryClient.request<BuyAndSellRatesResponse>(BUY_AND_SELL_RATES, {
			poolId: pool.poolId,
		})
		if (!response?.liquidityPools?.nodes) {
			throw new InvalidLiquidityIndexerResponseError("liquidityPools connection is missing")
		}
		const rates = response.liquidityPools.nodes[0]
		if (!rates) return undefined

		return {
			...pool,
			sourceChain: params.sourceChain,
			destinationChain: params.destinationChain,
			sellRate: rates.sellRate === null ? null : formatIndexerAmount(rates.sellRate, "sellRate"),
			buyRate: rates.buyRate === null ? null : formatIndexerAmount(rates.buyRate, "buyRate"),
			lastUpdatedAt: readIndexerDate(rates.lastUpdatedAt, "pool lastUpdatedAt"),
		}
	}
}

export class InvalidLiquidityIndexerResponseError extends Error {
	constructor(reason: string) {
		super(`Invalid liquidity indexer response: ${reason}`)
		this.name = "InvalidLiquidityIndexerResponseError"
	}
}

export class UnsupportedLiquidityAssetError extends Error {
	constructor(chain: string, asset: string) {
		super(`No configured liquidity asset found for ${asset} on ${chain}`)
		this.name = "UnsupportedLiquidityAssetError"
	}
}

export class UnsupportedLiquidityChainError extends Error {
	constructor(chainId: number | string) {
		super(`No configured liquidity chain found for chain ID ${chainId}`)
		this.name = "UnsupportedLiquidityChainError"
	}
}

function readLiquiditySlice(depth: string, providerCount: number, label: string): LiquiditySlice {
	if (!Number.isSafeInteger(providerCount) || providerCount < 0) {
		throw new InvalidLiquidityIndexerResponseError(`${label} provider count is invalid`)
	}
	return { totalLiquidity: formatIndexerAmount(depth, `${label} depth`), providerCount }
}

function formatIndexerAmount(value: string, label: string): string {
	try {
		const amount = BigInt(value)
		if (amount < 0n) throw new Error()
		return formatUnits(amount, POOL_DEPTH_DECIMALS)
	} catch {
		throw new InvalidLiquidityIndexerResponseError(`${label} is not a non-negative integer`)
	}
}

function readIndexerDate(value: string, label: string): Date {
	const date = new Date(dateStringtoTimestamp(value))
	if (Number.isNaN(date.getTime())) throw new InvalidLiquidityIndexerResponseError(`${label} is invalid`)
	return date
}
