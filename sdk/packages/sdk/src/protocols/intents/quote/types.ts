import type { PublicClient } from "viem"
import type { HexString } from "@/types"

export type IntentQuoteStrategy = "uniswap_v4"
export type IntentQuoteTradeType = "EXACT_INPUT" | "EXACT_OUTPUT"

/**
 * Token metadata required by intent quote strategies.
 */
export interface IntentQuoteToken {
	/** Token contract address on the user/source side. */
	address: HexString
	/** Token decimals for raw amount formatting by the caller. */
	decimals: number
	/** Optional token symbol used only to match SDK-configured destination pools. */
	symbol?: string
}

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
		 * `tokenIn.address`, which only works for same-chain quotes; pass this
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
 * `strategy` defaults to `uniswap_v4`, which is currently the only supported
 * strategy. Provide exactly one of `amountIn` or `amountOut`.
 */
export interface QuoteIntentParams {
	strategy?: IntentQuoteStrategy
	tokenIn: IntentQuoteToken
	tokenOut: IntentQuoteToken
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
	poolKey: UniswapV4PoolKey
	quoterAddress: HexString
	/**
	 * IntentGateway protocol fee on the source chain. The gateway deducts this
	 * from order inputs; callers should account for it alongside their own
	 * slippage tolerance when constructing the order.
	 */
	protocolFeeBps: bigint
}

/**
 * Quote data partners need before constructing an IntentGateway V2 order.
 *
 * `amountIn` and `amountOut` are the raw amounts returned by the Uniswap V4
 * quoter, with no slippage or protocol fee adjustments applied — callers apply
 * their own tolerance before placing the order.
 */
export interface QuoteIntentResult {
	strategy: "uniswap_v4"
	tradeType: IntentQuoteTradeType
	amountIn: bigint
	amountOut: bigint
	quoteMetadata: UniswapV4IntentQuoteMetadata
}

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
		tokenIn: IntentQuoteToken
		tokenOut: IntentQuoteToken
	}) {
		super(
			`No Uniswap v4 pool config found for ${params.tokenIn.symbol ?? params.tokenIn.address} -> ${
				params.tokenOut.symbol ?? params.tokenOut.address
			} on ${params.source} -> ${params.destination}`,
		)
		this.name = "UnsupportedIntentQuotePairError"
	}
}
