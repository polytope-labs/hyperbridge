import type { Chains, ConfiguredAssetSymbol } from "@/configs/chain"
import type { HexString } from "@/types"
import type { OrderbookRouteKind, OrderbookSide } from "./client"

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
	/**
	 * Return the orderbook's optimistic quote (`QuoteIntentResult`): one leg per
	 * order, each at its own price. Defaults to the pessimistic quote
	 * (`PessimisticQuoteIntentResult`): the whole trade at one price.
	 */
	optimistic?: boolean
}

/**
 * One order an optimistic intent quote takes, at the order's own price. Amounts
 * are raw token units; the rate stays at 1e18.
 */
export interface IntentQuoteLeg {
	/** The order's full advertised size in `tokenOut`, not just the part this leg takes. */
	advertisedSize: bigint
	/** The order's own price, quote per 1 base at 1e18, before the protocol fee: what this leg settles at. */
	orderRate: bigint
	/** The `tokenIn` this leg takes. The legs' inputs sum to the quote's `amountIn`. */
	amountIn: bigint
	/** The `tokenOut` this leg delivers, with the destination's protocol fee already taken off. */
	amountOut: bigint
}

/**
 * `quoteIntent`'s result with `optimistic: true`, the orderbook's optimistic `quote`: the trade split across the
 * route's orders, best price first, each at its own price. There is no single
 * rate or total output; the legs are the quote, and their `amountOut`s sum to
 * what the order should require. Amounts are raw token units.
 */
export interface QuoteIntentResult {
	/** `SAME_CHAIN`, or `CROSS_CHAIN` when only orders accepting the source chain can fill it. */
	route: OrderbookRouteKind
	/** `BID` when the order sells the book's base token, `ASK` when it buys it. */
	side: OrderbookSide
	/** The `tokenIn` priced: the requested input, or the input found to deliver the requested output. */
	amountIn: bigint
	/** The destination's protocol fee in basis points, already taken off every leg's `amountOut`. */
	slippageBps: number
	/**
	 * False when the route cannot fill the trade: `legs` is empty and `maxFillableIn` says how much
	 * it could. For an exact output, `amountIn` is then the input the SDK last tried.
	 */
	fillable: boolean
	/** The largest `amountIn` the route's orders could take together, each at its own price. */
	maxFillableIn: bigint
	/** The orders used, best price first. */
	legs: IntentQuoteLeg[]
}

/**
 * `quoteIntent`'s default result, the orderbook's `quotePessimistic`: the whole trade at one
 * price, the worst single-order price of the first level, best first, deep
 * enough to fill it by itself, or the route's worst price when no one level
 * can. Amounts are raw token units; rates stay at 1e18.
 */
export interface PessimisticQuoteIntentResult {
	route: OrderbookRouteKind
	side: OrderbookSide
	/** The `tokenIn` priced: the requested input, or the input found to deliver the requested output. */
	amountIn: bigint
	/** The `tokenOut` delivered at `rate`, with the destination's protocol fee already taken off. */
	amountOut: bigint
	/** Quote per 1 base, before the protocol fee: every order in the level fills at it. */
	rate: bigint | null
	/** The price bucket of that level. */
	priceBucket: bigint | null
	/** The destination's protocol fee in basis points, already taken off `amountOut`. */
	slippageBps: number
	/**
	 * False when the route cannot fill the trade: `amountOut` is 0, `rate` and `priceBucket` are
	 * null, and `maxFillableIn` says how much it could. For an exact output, `amountIn` is then the
	 * input the SDK last tried.
	 */
	fillable: boolean
	/** The largest `amountIn` this quote could fill: any one level at its worst price, or the whole route at its worst. */
	maxFillableIn: bigint
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
