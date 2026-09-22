import type { Chains, ConfiguredAssetSymbol } from "@/configs/chain"
import type { HexString } from "@/types"
import type { OrderbookRouteKind, OrderbookSide } from "./client"

export type IntentQuoteTradeType = "EXACT_INPUT" | "EXACT_OUTPUT"

/**
 * Parameters for `IntentGateway.quoteIntent`. The source and destination chains
 * come from the gateway instance itself. `tokenIn` and `tokenOut` are token
 * addresses; the SDK resolves their configured symbols and decimals. Provide
 * exactly one amount, in the token's raw units.
 */
export interface QuoteIntentParams {
	/** Token address on the source chain. */
	tokenIn: HexString
	/** Token address on the destination chain. */
	tokenOut: HexString
	amountIn?: bigint
	amountOut?: bigint
}

export interface IntentQuoteMetadata {
	sourceChain: Chains
	destinationChain: Chains
	/** `SAME_CHAIN`, or `CROSS_CHAIN` when only orders accepting the source chain can fill it. */
	route: OrderbookRouteKind
	/** `BID` when the order sells the book's base token, `ASK` when it buys it. */
	side: OrderbookSide
	baseTokenSymbol: ConfiguredAssetSymbol
	quoteTokenSymbol: ConfiguredAssetSymbol
	/** The clearing price the order was quoted at, in quote-token units per one base token. */
	rate: string
	/** The largest `amountIn` the route can fill right now, in the source token's raw units. */
	maxFillableIn: bigint
	/** How many orders the quote combines. */
	orderCount: number
}

/**
 * A quote priced from the HyperFX orderbook.
 *
 * `amountIn` and `amountOut` are raw token units and can be used directly as
 * the order's `inputs` and `output.assets`. The orderbook's rates already carry
 * the IntentGateway protocol fee, so no further fee adjustment is needed.
 */
export interface QuoteIntentResult {
	tradeType: IntentQuoteTradeType
	amountIn: bigint
	amountOut: bigint
	quoteMetadata: IntentQuoteMetadata
}

/**
 * What the orderbook holds for one route, with no amount given. Amounts are
 * decimal strings in whole tokens.
 */
export interface AvailableLiquidity {
	sourceChain: Chains
	destinationChain: Chains
	tokenInSymbol: ConfiguredAssetSymbol
	tokenOutSymbol: ConfiguredAssetSymbol
	/** `tokenOut` on the destination chain. */
	tokenAddress: HexString
	route: OrderbookRouteKind
	side: OrderbookSide
	baseTokenSymbol: ConfiguredAssetSymbol
	quoteTokenSymbol: ConfiguredAssetSymbol
	/** The best rate on the route in quote-token units per one base token, or `null` when no order serves it. */
	bestRate: string | null
	/** True liquidity in `tokenOut`: each serving solver's largest balance-checked order, summed. */
	availableLiquidity: string
	/** Virtual depth: every serving order's size summed, in `tokenIn` and `tokenOut`. Can exceed what solvers hold. */
	depthIn: string
	depthOut: string
	/** The largest `tokenIn` amount the route could fill with its orders combined. */
	maxFillableIn: string
	orderCount: number
	solverCount: number
}

/**
 * The best bid and ask for a pair on a route, in quote-token units per one base
 * token, as decimal strings. Either side is `null` while it has no orders.
 */
export interface BuyAndSellRates {
	baseTokenSymbol: ConfiguredAssetSymbol
	quoteTokenSymbol: ConfiguredAssetSymbol
	sourceChain: Chains
	destinationChain: Chains
	/** Highest bid: the quote token a seller receives per one base token. */
	bid: string | null
	/** Lowest ask: the quote token a buyer pays per one base token. */
	ask: string | null
	/** Midpoint of `bid` and `ask`; `null` unless both sides have orders. */
	mid: string | null
	/** `ask - bid`; `null` when a side is empty or the book is crossed. */
	spread: string | null
	/** `spread` over `mid` in basis points, to two decimal places. */
	spreadBps: number | null
}

export class UnsupportedLiquidityAssetError extends Error {
	constructor(chain: string, asset: string) {
		super(`No configured asset found for ${asset} on ${chain}`)
		this.name = "UnsupportedLiquidityAssetError"
	}
}

export class UnsupportedLiquidityChainError extends Error {
	constructor(chainId: number | string) {
		super(`No configured chain found for chain ID ${chainId}`)
		this.name = "UnsupportedLiquidityChainError"
	}
}

/**
 * An exact-output quote did not settle on an input within its rounds: each quoted input's clearing
 * price still fell short of the output. The route may have the depth; the book moved or its levels
 * are too steep to converge on in time. Retrying, or quoting an exact input, may succeed.
 */
export class OrderbookQuoteNotConvergedError extends Error {
	constructor(
		readonly route: { tokenIn: string; tokenOut: string; sourceChain: string; destinationChain: string },
		readonly rounds: number,
		/** The last input quoted, in the source token's raw units. */
		readonly lastAmountIn: bigint,
	) {
		super(
			`No input for ${route.tokenIn} -> ${route.tokenOut} on ${route.sourceChain} -> ${route.destinationChain} delivered the requested output within ${rounds} quotes; the last tried ${lastAmountIn} raw units`,
		)
		this.name = "OrderbookQuoteNotConvergedError"
	}
}

/** The orderbook cannot fill the requested amount on this route. */
export class InsufficientOrderbookLiquidityError extends Error {
	constructor(
		readonly route: { tokenIn: string; tokenOut: string; sourceChain: string; destinationChain: string },
		/** The largest `amountIn` the route can fill, in the source token's raw units. */
		readonly maxFillableIn: bigint,
	) {
		super(
			`The HyperFX orderbook cannot fill ${route.tokenIn} -> ${route.tokenOut} on ${route.sourceChain} -> ${route.destinationChain}; max fillable input is ${maxFillableIn} raw units`,
		)
		this.name = "InsufficientOrderbookLiquidityError"
	}
}
