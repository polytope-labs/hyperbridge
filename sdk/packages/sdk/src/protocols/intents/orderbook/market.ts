import { formatUnits } from "viem"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import { type Chains, type ConfiguredAssetSymbol, getConfigByStateMachineId } from "@/configs/chain"
import type { HexString } from "@/types"
import {
	type HyperFxOrderbook,
	ORDERBOOK_DECIMALS,
	ORDERBOOK_SCALE,
	OrderbookRequestError,
	type OrderbookRoute,
	type OrderbookSide,
	type OrderbookSwapQuote,
} from "./client"
import {
	type AvailableLiquidity,
	type BuyAndSellRates,
	InsufficientOrderbookLiquidityError,
	type IntentQuoteTradeType,
	OrderbookQuoteNotConvergedError,
	type QuoteIntentParams,
	type QuoteIntentResult,
	UnsupportedLiquidityAssetError,
	UnsupportedLiquidityChainError,
} from "./types"

/**
 * Round trips an exact-output quote may take. Each one quotes a larger input at
 * the clearing price the last one found, which only worsens with size, so a
 * route with the depth converges in a few.
 */
const MAX_EXACT_OUTPUT_ROUNDS = 16

interface ResolvedAsset {
	chain: Chains
	symbol: ConfiguredAssetSymbol
	address: HexString
	decimals: number
}

/**
 * Prices intents, liquidity and rates from the HyperFX orderbook.
 *
 * Callers name tokens by address or symbol on a configured chain, in raw
 * units; the orderbook names them by symbol at 1e18. This is the translation.
 */
export class OrderbookMarket {
	constructor(
		private readonly configService: ChainConfigService,
		private readonly orderbook: () => HyperFxOrderbook,
	) {}

	async quoteIntent(
		params: QuoteIntentParams,
		sourceChain: string,
		destinationChain: string,
	): Promise<QuoteIntentResult> {
		validateQuoteParams(params)
		const tokenIn = this.assetByAddress(sourceChain, params.tokenIn)
		const tokenOut = this.assetByAddress(destinationChain, params.tokenOut)
		const route = toRoute(tokenIn, tokenOut)

		if (params.amountIn !== undefined) {
			const quote = await this.orderbook().quote(route, toOrderbookAmount(params.amountIn, tokenIn.decimals))
			const amountOut = quote.fillable ? fromOrderbookAmount(quote.amountOut, tokenOut.decimals) : 0n
			if (amountOut === 0n) throw this.insufficient(route, quote.maxFillableIn, tokenIn)
			return buildQuote("EXACT_INPUT", params.amountIn, amountOut, quote, tokenIn, tokenOut)
		}

		const amountOut = params.amountOut as bigint
		const quote = await this.quoteExactOutput(route, toOrderbookAmount(amountOut, tokenOut.decimals), tokenIn)
		return buildQuote(
			"EXACT_OUTPUT",
			fromOrderbookAmount(quote.amountIn, tokenIn.decimals),
			amountOut,
			quote,
			tokenIn,
			tokenOut,
		)
	}

	async availableLiquidity(
		params: Pick<QuoteIntentParams, "tokenIn" | "tokenOut">,
		sourceChain: string,
		destinationChain: string,
	): Promise<AvailableLiquidity> {
		const tokenIn = this.assetByAddress(sourceChain, params.tokenIn)
		const tokenOut = this.assetByAddress(destinationChain, params.tokenOut)
		const liquidity = await this.orderbook().routeLiquidity(toRoute(tokenIn, tokenOut))
		const [base, quote] = liquidity.side === "BID" ? [tokenIn, tokenOut] : [tokenOut, tokenIn]
		return {
			sourceChain: tokenIn.chain,
			destinationChain: tokenOut.chain,
			tokenInSymbol: tokenIn.symbol,
			tokenOutSymbol: tokenOut.symbol,
			tokenAddress: tokenOut.address,
			route: liquidity.route,
			side: liquidity.side,
			baseTokenSymbol: base.symbol,
			quoteTokenSymbol: quote.symbol,
			bestRate: formatOptional(liquidity.bestRate),
			availableLiquidity: format(liquidity.availableLiquidity),
			depthIn: format(liquidity.depthIn),
			depthOut: format(liquidity.depthOut),
			maxFillableIn: format(liquidity.maxFillableIn),
			orderCount: liquidity.orderCount,
			solverCount: liquidity.solverCount,
		}
	}

	async buyAndSellRates(params: {
		sourceChain: string
		destinationChain: string
		tokenInSymbol: string
		tokenOutSymbol: string
	}): Promise<BuyAndSellRates> {
		const tokenIn = this.assetBySymbol(params.sourceChain, params.tokenInSymbol)
		const tokenOut = this.assetBySymbol(params.destinationChain, params.tokenOutSymbol)
		const { book, bid, ask } = await this.orderbook().topOfBook({
			tokenA: tokenIn.symbol,
			tokenB: tokenOut.symbol,
			fillChain: tokenOut.chain,
			sourceChain: tokenIn.chain,
		})

		const bidRate = bid?.rate ?? null
		const askRate = ask?.rate ?? null
		const mid = bidRate !== null && askRate !== null ? (bidRate + askRate) / 2n : null
		const spread = bidRate !== null && askRate !== null && askRate >= bidRate ? askRate - bidRate : null
		return {
			baseTokenSymbol: book.base as ConfiguredAssetSymbol,
			quoteTokenSymbol: book.quote as ConfiguredAssetSymbol,
			sourceChain: tokenIn.chain,
			destinationChain: tokenOut.chain,
			bid: formatOptional(bidRate),
			ask: formatOptional(askRate),
			mid: formatOptional(mid),
			spread: formatOptional(spread),
			spreadBps: spread !== null && mid ? Number((spread * 1_000_000n) / mid) / 100 : null,
		}
	}

	/**
	 * The orderbook only quotes an input amount, so an output is priced by
	 * guessing the input from the best rate and raising it at each clearing
	 * price until the quote delivers `targetOut`.
	 */
	private async quoteExactOutput(
		route: OrderbookRoute,
		targetOut: bigint,
		tokenIn: ResolvedAsset,
	): Promise<OrderbookSwapQuote> {
		const orderbook = this.orderbook()
		const liquidity = await orderbook.routeLiquidity(route)
		if (liquidity.bestRate === null) throw this.insufficient(route, 0n, tokenIn)

		const inputUnit = toOrderbookAmount(1n, tokenIn.decimals)
		let amountIn = roundUpTo(requiredInput(liquidity.side, targetOut, liquidity.bestRate), inputUnit)
		let lastAmountIn = amountIn
		for (let round = 0; round < MAX_EXACT_OUTPUT_ROUNDS; round++) {
			// Past the most the route can fill, no input delivers the output.
			if (amountIn > liquidity.maxFillableIn) throw this.insufficient(route, liquidity.maxFillableIn, tokenIn)
			const quote = await orderbook.quote(route, amountIn)
			if (quote.fillable && quote.amountOut >= targetOut) return quote
			lastAmountIn = amountIn
			const next =
				quote.rate === null ? amountIn : roundUpTo(requiredInput(quote.side, targetOut, quote.rate), inputUnit)
			amountIn = next > amountIn ? next : amountIn + inputUnit
		}
		throw new OrderbookQuoteNotConvergedError(
			route,
			MAX_EXACT_OUTPUT_ROUNDS,
			fromOrderbookAmount(lastAmountIn, tokenIn.decimals),
		)
	}

	private insufficient(
		route: OrderbookRoute,
		maxFillableIn: bigint,
		tokenIn: ResolvedAsset,
	): InsufficientOrderbookLiquidityError {
		return new InsufficientOrderbookLiquidityError(route, fromOrderbookAmount(maxFillableIn, tokenIn.decimals))
	}

	private assetByAddress(chain: string, address: HexString): ResolvedAsset {
		const resolvedChain = resolveChain(chain)
		return withDecimals(
			resolvedChain,
			address,
			this.configService.getAssetMetadataByAddress(resolvedChain, address),
		)
	}

	private assetBySymbol(chain: string, symbol: string): ResolvedAsset {
		const resolvedChain = resolveChain(chain)
		return withDecimals(resolvedChain, symbol, this.configService.getAssetMetadataBySymbol(resolvedChain, symbol))
	}
}

function resolveChain(chain: string): Chains {
	const stateMachineId = getConfigByStateMachineId(chain)?.stateMachineId
	if (!stateMachineId) throw new UnsupportedLiquidityChainError(chain)
	return stateMachineId
}

function withDecimals(
	chain: Chains,
	asset: string,
	metadata: { symbol: ConfiguredAssetSymbol; address: HexString; decimals?: number } | undefined,
): ResolvedAsset {
	if (!metadata) throw new UnsupportedLiquidityAssetError(chain, asset)
	const { decimals } = metadata
	if (decimals === undefined || !Number.isSafeInteger(decimals) || decimals < 0 || decimals > ORDERBOOK_DECIMALS) {
		throw new UnsupportedLiquidityAssetError(chain, `${metadata.symbol} (decimals are not configured)`)
	}
	return { chain, symbol: metadata.symbol, address: metadata.address, decimals }
}

function validateQuoteParams(params: QuoteIntentParams): void {
	const hasAmountIn = params.amountIn !== undefined
	const hasAmountOut = params.amountOut !== undefined
	if (hasAmountIn === hasAmountOut) throw new Error("Provide exactly one of amountIn or amountOut")
	if (params.amountIn !== undefined && params.amountIn <= 0n) throw new Error("amountIn must be greater than zero")
	if (params.amountOut !== undefined && params.amountOut <= 0n) throw new Error("amountOut must be greater than zero")
}

function toRoute(tokenIn: ResolvedAsset, tokenOut: ResolvedAsset): OrderbookRoute {
	return {
		tokenIn: tokenIn.symbol,
		tokenOut: tokenOut.symbol,
		sourceChain: tokenIn.chain,
		destinationChain: tokenOut.chain,
	}
}

/** The tokenIn (1e18) that buys `amountOut` at `rate`, quote per 1 base, rounded up. */
function requiredInput(side: OrderbookSide, amountOut: bigint, rate: bigint): bigint {
	// A bid takes base and pays quote; an ask takes quote and pays base.
	return side === "BID" ? divCeil(amountOut * ORDERBOOK_SCALE, rate) : divCeil(amountOut * rate, ORDERBOOK_SCALE)
}

function buildQuote(
	tradeType: IntentQuoteTradeType,
	amountIn: bigint,
	amountOut: bigint,
	quote: OrderbookSwapQuote,
	tokenIn: ResolvedAsset,
	tokenOut: ResolvedAsset,
): QuoteIntentResult {
	if (quote.rate === null) throw new OrderbookRequestError("the orderbook served a fillable quote with no rate")
	const [base, quoteToken] = quote.side === "BID" ? [tokenIn, tokenOut] : [tokenOut, tokenIn]
	return {
		tradeType,
		amountIn,
		amountOut,
		quoteMetadata: {
			sourceChain: tokenIn.chain,
			destinationChain: tokenOut.chain,
			route: quote.route,
			side: quote.side,
			baseTokenSymbol: base.symbol,
			quoteTokenSymbol: quoteToken.symbol,
			rate: format(quote.rate),
			maxFillableIn: fromOrderbookAmount(quote.maxFillableIn, tokenIn.decimals),
			orderCount: quote.fills.length,
		},
	}
}

/** A raw token amount at the orderbook's 1e18 scale. */
export function toOrderbookAmount(amount: bigint, decimals: number): bigint {
	return amount * 10n ** BigInt(ORDERBOOK_DECIMALS - decimals)
}

/** A 1e18 orderbook amount in raw token units, rounded down. */
export function fromOrderbookAmount(amount: bigint, decimals: number): bigint {
	return amount / 10n ** BigInt(ORDERBOOK_DECIMALS - decimals)
}

function roundUpTo(amount: bigint, unit: bigint): bigint {
	return divCeil(amount, unit) * unit
}

function divCeil(numerator: bigint, denominator: bigint): bigint {
	return (numerator + denominator - 1n) / denominator
}

function format(amount: bigint): string {
	return formatUnits(amount, ORDERBOOK_DECIMALS)
}

function formatOptional(amount: bigint | null): string | null {
	return amount === null ? null : format(amount)
}
