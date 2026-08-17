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
	direct: RateConnection
	reverse: RateConnection
}

interface RateConnection {
	nodes: ChainRateNode[]
}

interface ChainRateNode {
	rate: string
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
	 * Returns chain-specific buy and sell rates in less-valued quote-token units
	 * per one base token.
	 *
	 * The requested direction is read on the destination chain; its reverse is
	 * read on the source chain. This mirrors where each direction's output token
	 * must be delivered for a cross-chain trade.
	 */
	async getBuyAndSellRates(params: {
		sourceChain: Chains
		destinationChain: Chains
		tokenInSymbol: ConfiguredAssetSymbol
		tokenOutSymbol: ConfiguredAssetSymbol
	}): Promise<BuyAndSellRates | undefined> {
		const pool = resolveLiquidityPool(params.tokenInSymbol, params.tokenOutSymbol)
		const directDirection: PoolDirection =
			params.tokenInSymbol.toLowerCase() === pool.token0Symbol.toLowerCase() ? SELL : BUY
		const reverseDirection: PoolDirection = directDirection === SELL ? BUY : SELL
		const response = await this.queryClient.request<BuyAndSellRatesResponse>(BUY_AND_SELL_RATES, {
			poolId: pool.poolId,
			directChain: params.destinationChain,
			directDirection,
			reverseChain: params.sourceChain,
			reverseDirection,
		})
		if (!response?.direct?.nodes || !response?.reverse?.nodes) {
			throw new InvalidLiquidityIndexerResponseError("rate connections are missing")
		}

		const direct = readIndexedRate(response.direct.nodes[0], "direct rate")
		const reverse = readIndexedRate(response.reverse.nodes[0], "reverse rate")
		if (!direct && !reverse) return undefined

		const quoteTokenSymbol = resolveQuoteTokenSymbol(
			params.tokenInSymbol,
			params.tokenOutSymbol,
			direct?.scaledRate,
			reverse?.scaledRate,
		)
		const quoteIsTokenOut = quoteTokenSymbol === params.tokenOutSymbol
		const buy = quoteIsTokenOut ? direct : reverse
		const sell = quoteIsTokenOut ? reverse : direct

		return {
			baseTokenSymbol: quoteIsTokenOut ? params.tokenInSymbol : params.tokenOutSymbol,
			quoteTokenSymbol,
			sourceChain: params.sourceChain,
			destinationChain: params.destinationChain,
			buyRate: buy ? formatUnits(buy.scaledRate, INDEXER_FIXED_POINT_DECIMALS) : null,
			sellRate: sell
				? formatUnits(reciprocalRate(sell.scaledRate, "sell rate"), INDEXER_FIXED_POINT_DECIMALS)
				: null,
			buyRateUpdatedAt: buy?.updatedAt ?? null,
			sellRateUpdatedAt: sell?.updatedAt ?? null,
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

function readIndexedRate(node: ChainRateNode | undefined, label: string): IndexedRate | undefined {
	if (!node) return undefined
	try {
		const scaledRate = BigInt(node.rate)
		if (scaledRate <= 0n) throw new Error()
		return {
			scaledRate,
			updatedAt: readIndexerDate(node.lastUpdatedAt, `${label} lastUpdatedAt`),
		}
	} catch (error) {
		if (error instanceof InvalidLiquidityIndexerResponseError) throw error
		throw new InvalidLiquidityIndexerResponseError(`${label} is not a positive integer`)
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
	const reciprocal = (POOL_RATE_SCALE * POOL_RATE_SCALE) / rate
	if (reciprocal <= 0n) throw new InvalidLiquidityIndexerResponseError(`${label} reciprocal underflowed`)
	return reciprocal
}
