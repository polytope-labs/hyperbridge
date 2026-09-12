import type { Chains, ConfiguredAssetSymbol } from "@/configs/chain"
import type { HexString } from "@/types"
import type { PublicClient } from "viem"

export type IntentQuoteStrategy = "indexed_rates" | "uniswap_v4" | "phantom_snapshot"
export type IntentQuoteTradeType = "EXACT_INPUT" | "EXACT_OUTPUT"
export type IndexedRateSide = "buy" | "sell"

/**
 * Full Uniswap V4 PoolKey. V4 pools cannot be discovered from a token pair alone.
 */
export interface UniswapV4PoolKey {
	currency0: HexString
	currency1: HexString
	fee: number
	tickSpacing: number
	hooks: HexString
}

export interface UniswapV4IntentQuoteOptions {
	/** Explicit pool override for pairs not yet present in SDK chain config. */
	poolKey?: UniswapV4PoolKey & {
		quoterAddress?: HexString
		/**
		 * Destination-side address of the input currency. Defaults to
		 * `tokenIn`, which only works for same-chain quotes; pass this
		 * explicitly when source and destination chains differ. Must equal
		 * `currency0` or `currency1`.
		 */
		currencyIn?: HexString
	}
}

/**
 * Parameters for `IntentGateway.quoteIntent`. The source and destination
 * chains come from the gateway instance itself.
 *
 * Quotes default to the aggregate pool's `indexed_rates`. Legacy Phantom
 * snapshots and Uniswap V4 remain available as explicit strategies. `tokenIn`
 * and `tokenOut` are token addresses; the SDK resolves configured token
 * metadata and decimals internally. Provide exactly one amount.
 */
export interface QuoteIntentParams {
	strategy?: IntentQuoteStrategy
	/** Token address on the source chain. */
	tokenIn: HexString
	/** Token address on the destination chain. */
	tokenOut: HexString
	amountIn?: bigint
	amountOut?: bigint
	uniswapV4?: UniswapV4IntentQuoteOptions
}

/** Chain connection handed to quote strategies by the IntentGateway. */
export interface IntentQuoteChainContext {
	stateMachineId: string
	client: PublicClient
}

export interface UniswapV4IntentQuoteMetadata {
	/** Chain whose Uniswap pool and quoter were used for the quote. */
	quoteChain: string
	poolKey: UniswapV4PoolKey
	quoterAddress: HexString
	/**
	 * IntentGateway protocol fee on the source chain, already applied to the
	 * returned amounts. Exposed so callers can reconstruct the swap-side
	 * (post-fee) amount if needed.
	 */
	protocolFeeBps: bigint
}

export interface PhantomSnapshotIntentQuoteMetadata {
	/** Canonical chain whose directional Phantom pair addresses identify the feed. */
	quoteChain: string
	/** Phantom order whose bids produced this snapshot. */
	commitment: HexString
	tokenA: HexString
	tokenB: HexString
	/** Benchmark input and liquidity-weighted median output, both in raw token units. */
	standardAmount: bigint
	medianPrice: bigint
	lowestPrice?: bigint
	highestPrice?: bigint
	blockNumber: bigint
	snapshotTime: Date
	bidCount: number
	/** Source gateway protocol fee already reflected in the returned quote amounts. */
	protocolFeeBps: bigint
}

export interface IndexedRateIntentQuoteMetadata {
	sourceChain: Chains
	destinationChain: Chains
	baseTokenSymbol: ConfiguredAssetSymbol
	quoteTokenSymbol: ConfiguredAssetSymbol
	/** Directional pool rate used for this order. */
	rateSide: IndexedRateSide
	/** Quote-token units per one base token. */
	rate: string
	rateUpdatedAt: Date
	/** Source gateway protocol fee already reflected in the returned quote amounts. */
	protocolFeeBps: bigint
}

/**
 * Quote data partners need before constructing an IntentGateway V2 order.
 *
 * `amountIn` and `amountOut` already account for the IntentGateway protocol
 * fee that the gateway deducts from order inputs (see `quoteMetadata`). No
 * further fee or slippage adjustment is required before placing the order.
 */
export interface UniswapV4QuoteIntentResult {
	strategy: "uniswap_v4"
	tradeType: IntentQuoteTradeType
	amountIn: bigint
	amountOut: bigint
	quoteMetadata: UniswapV4IntentQuoteMetadata
}

export interface PhantomSnapshotQuoteIntentResult {
	strategy: "phantom_snapshot"
	tradeType: IntentQuoteTradeType
	amountIn: bigint
	amountOut: bigint
	quoteMetadata: PhantomSnapshotIntentQuoteMetadata
}

export interface IndexedRateQuoteIntentResult {
	strategy: "indexed_rates"
	tradeType: IntentQuoteTradeType
	amountIn: bigint
	amountOut: bigint
	quoteMetadata: IndexedRateIntentQuoteMetadata
}

export type QuoteIntentResult =
	| IndexedRateQuoteIntentResult
	| UniswapV4QuoteIntentResult
	| PhantomSnapshotQuoteIntentResult

export interface IntentQuoteStrategyHandler {
	quote(
		params: QuoteIntentParams,
		source: IntentQuoteChainContext,
		destination: IntentQuoteChainContext,
	): Promise<QuoteIntentResult>
}

export class UnsupportedIntentQuoteStrategyError extends Error {
	constructor(strategy: string) {
		super(`Unsupported intent quote strategy: ${strategy}`)
		this.name = "UnsupportedIntentQuoteStrategyError"
	}
}

export class UnsupportedIntentQuotePairError extends Error {
	constructor(params: {
		source: string
		destination: string
		tokenIn: HexString
		tokenOut: HexString
		quoteSource?: string
	}) {
		super(
			`No ${params.quoteSource ?? "Uniswap v4 pool config"} found for ${params.tokenIn} -> ${params.tokenOut} on ${params.source} -> ${params.destination}`,
		)
		this.name = "UnsupportedIntentQuotePairError"
	}
}

export class PhantomSnapshotUnavailableError extends Error {
	constructor(tokenA: HexString, tokenB: HexString) {
		super(`No Phantom order price snapshot found for ${tokenA} -> ${tokenB}`)
		this.name = "PhantomSnapshotUnavailableError"
	}
}

export class InvalidPhantomSnapshotError extends Error {
	constructor(commitment: HexString, reason: string) {
		super(`Invalid Phantom order price snapshot ${commitment}: ${reason}`)
		this.name = "InvalidPhantomSnapshotError"
	}
}

export class IndexedRateUnavailableError extends Error {
	constructor(params: {
		source?: string
		destination?: string
		tokenIn?: string
		tokenOut?: string
		side?: IndexedRateSide
	}) {
		const route =
			params.source && params.destination && params.tokenIn && params.tokenOut
				? ` for ${params.tokenIn} -> ${params.tokenOut} on ${params.source} -> ${params.destination}`
				: ""
		const side = params.side ? ` ${params.side}` : ""
		super(`No indexed${side} rate available${route}`)
		this.name = "IndexedRateUnavailableError"
	}
}

export class InvalidIndexedRateError extends Error {
	constructor(reason: string) {
		super(`Invalid indexed intent rate: ${reason}`)
		this.name = "InvalidIndexedRateError"
	}
}
