import type { Chains, ConfiguredAssetSymbol } from "@/configs/chain"
import { AVAILABLE_LIQUIDITY, BUY_AND_SELL_RATES } from "@/queries"
import type { AvailableLiquidity, BuyAndSellRates, HexString, IndexerQueryClient, LiquiditySlice } from "@/types"
import { dateStringtoTimestamp, normalizeEvmAddress } from "@/utils"
import { formatUnits } from "viem"
import { resolveLiquidityPool } from "./liquidity-pool"

const INDEXER_FIXED_POINT_DECIMALS = 18
const POOL_RATE_SCALE = 10n ** 18n
const SELL = "SELL"
const BUY = "BUY"
const USD_STABLE_SYMBOLS = new Set<ConfiguredAssetSymbol>(["USDC", "USDT"])

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
		nodes: LiquidityPoolRateNode[]
	}
}

interface LiquidityPoolRateNode {
	id: string
	token0Symbol: string
	token1Symbol: string
	sellRate: string | null
	buyRate: string | null
	lastUpdatedAt: string
}

interface IndexedRate {
	scaledRate: bigint
	updatedAt: Date
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

	/**
	 * Returns the indexed pool's aggregate buy and sell rates in less-valued
	 * quote-token units per one base token.
	 *
	 * The indexer depth-weights fresh per-chain samples into the pool rates. The
	 * source and destination chains remain part of the result because they define
	 * the cross-chain route whose configured token symbols were resolved.
	 */
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
			throw new InvalidLiquidityIndexerResponseError("liquidity pool connection is missing")
		}
		const indexedPool = response.liquidityPools.nodes[0]
		if (!indexedPool) return undefined
		validateIndexedPool(indexedPool, pool)

		const sell = readIndexedRate(indexedPool.sellRate, indexedPool.lastUpdatedAt, "pool sell rate")
		const buy = readIndexedRate(indexedPool.buyRate, indexedPool.lastUpdatedAt, "pool buy rate")
		const inputIsToken0 = params.tokenInSymbol.toLowerCase() === pool.token0Symbol.toLowerCase()
		const direct = inputIsToken0 ? sell : buy
		const reverse = inputIsToken0 ? buy : sell
		if (!direct && !reverse) return undefined

		const quoteTokenSymbol = resolveQuoteTokenSymbol(
			params.tokenInSymbol,
			params.tokenOutSymbol,
			direct?.scaledRate,
			reverse?.scaledRate,
		)
		const quoteIsTokenOut = quoteTokenSymbol === params.tokenOutSymbol
		const orientedBuy = quoteIsTokenOut ? direct : reverse
		const orientedSell = quoteIsTokenOut ? reverse : direct

		return {
			baseTokenSymbol: quoteIsTokenOut ? params.tokenInSymbol : params.tokenOutSymbol,
			quoteTokenSymbol,
			sourceChain: params.sourceChain,
			destinationChain: params.destinationChain,
			buyRate: orientedBuy ? formatUnits(orientedBuy.scaledRate, INDEXER_FIXED_POINT_DECIMALS) : null,
			sellRate: orientedSell
				? formatUnits(reciprocalRate(orientedSell.scaledRate, "sell rate"), INDEXER_FIXED_POINT_DECIMALS)
				: null,
			buyRateUpdatedAt: orientedBuy?.updatedAt ?? null,
			sellRateUpdatedAt: orientedSell?.updatedAt ?? null,
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
		return formatUnits(amount, INDEXER_FIXED_POINT_DECIMALS)
	} catch {
		throw new InvalidLiquidityIndexerResponseError(`${label} is not a non-negative integer`)
	}
}

function readIndexerDate(value: string, label: string): Date {
	const date = new Date(dateStringtoTimestamp(value))
	if (Number.isNaN(date.getTime())) throw new InvalidLiquidityIndexerResponseError(`${label} is invalid`)
	return date
}

function readIndexedRate(value: string | null, lastUpdatedAt: string, label: string): IndexedRate | undefined {
	if (value === null) return undefined
	try {
		const scaledRate = BigInt(value)
		if (scaledRate <= 0n) throw new Error()
		return {
			scaledRate,
			updatedAt: readIndexerDate(lastUpdatedAt, `${label} lastUpdatedAt`),
		}
	} catch (error) {
		if (error instanceof InvalidLiquidityIndexerResponseError) throw error
		throw new InvalidLiquidityIndexerResponseError(`${label} is not a positive integer`)
	}
}

function validateIndexedPool(
	indexedPool: LiquidityPoolRateNode,
	expected: { poolId: string; token0Symbol: string; token1Symbol: string },
): void {
	if (
		indexedPool.id.toLowerCase() !== expected.poolId.toLowerCase() ||
		indexedPool.token0Symbol.toLowerCase() !== expected.token0Symbol.toLowerCase() ||
		indexedPool.token1Symbol.toLowerCase() !== expected.token1Symbol.toLowerCase()
	) {
		throw new InvalidLiquidityIndexerResponseError(`pool identity does not match ${expected.poolId}`)
	}
}

function resolveQuoteTokenSymbol(
	tokenInSymbol: ConfiguredAssetSymbol,
	tokenOutSymbol: ConfiguredAssetSymbol,
	directRate?: bigint,
	reverseRate?: bigint,
): ConfiguredAssetSymbol {
	const inputIsUsdStable = USD_STABLE_SYMBOLS.has(tokenInSymbol)
	const outputIsUsdStable = USD_STABLE_SYMBOLS.has(tokenOutSymbol)
	if (inputIsUsdStable !== outputIsUsdStable) return inputIsUsdStable ? tokenOutSymbol : tokenInSymbol

	if (directRate !== undefined) return directRate >= POOL_RATE_SCALE ? tokenOutSymbol : tokenInSymbol
	if (reverseRate !== undefined) return reverseRate >= POOL_RATE_SCALE ? tokenInSymbol : tokenOutSymbol

	throw new InvalidLiquidityIndexerResponseError("cannot orient an empty rate pair")
}

function reciprocalRate(rate: bigint, label: string): bigint {
	const numerator = POOL_RATE_SCALE * POOL_RATE_SCALE
	// A sell rate is quote token required per base token. Round the reciprocal
	// up so reverse quotes never promise more base token than the indexed
	// base-per-quote direction can deliver.
	const reciprocal = (numerator + rate - 1n) / rate
	if (reciprocal <= 0n) throw new InvalidLiquidityIndexerResponseError(`${label} reciprocal underflowed`)
	return reciprocal
}
