import { formatUnits } from "viem"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import { type Chains, type ConfiguredAssetSymbol, getConfigByStateMachineId } from "@/configs/chain"
import type { HexString } from "@/types"
import {
	type HyperFxOrderbook,
	ORDERBOOK_DECIMALS,
	ORDERBOOK_SCALE,
	OrderbookRequestError,
	type OrderbookLevelQuote,
	type OrderbookRoute,
	type OrderbookSide,
	type OrderbookSwapQuote,
} from "./client"
import {
	type AvailableLiquidity,
	type BuyAndSellRates,
	InsufficientOrderbookLiquidityError,
	OrderbookQuoteNotConvergedError,
	type PessimisticQuoteIntentResult,
	type QuoteIntentParams,
	type QuoteIntentResult,
	UnsupportedLiquidityAssetError,
	UnsupportedLiquidityChainError,
} from "./types"

/**
 * Round trips an exact-output quote may take. Each one re-prices the part of the
 * last one filled at its worst rate, which only worsens with size, so a route
 * with the depth converges in a few.
 */
const MAX_EXACT_OUTPUT_ROUNDS = 16

/** What pricing an intent reads from either orderbook quote. Amounts at 1e18. */
interface QuoteTotals {
	side: OrderbookSide
	amountIn: bigint
	/** The total tokenOut delivered, after the protocol fee; 0 when not fillable. */
	amountOut: bigint
	fillable: boolean
	maxFillableIn: bigint
	/**
	 * The input and output filled at the quote's worst price, output after the protocol fee: an
	 * optimistic quote's last fill, a pessimistic quote's whole trade. Their ratio prices more
	 * input at that price.
	 */
	worstIn: bigint
	worstOut: bigint
}

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

	/**
	 * Quotes an intent from the orderbook, in raw token units. By default this is
	 * `quotePessimistic`: one price, from the first level deep enough to fill the
	 * whole trade by itself, at which every order in it fills, or the route's
	 * worst price when no one level can. With `optimistic` it is the optimistic
	 * `quote`: the route's orders, best price first, each filling what it can at
	 * its own price, one leg per order.
	 */
	quoteIntent(
		params: QuoteIntentParams & { optimistic: true },
		sourceChain: string,
		destinationChain: string,
	): Promise<QuoteIntentResult>
	quoteIntent(
		params: QuoteIntentParams & { optimistic?: false },
		sourceChain: string,
		destinationChain: string,
	): Promise<PessimisticQuoteIntentResult>
	quoteIntent(
		params: QuoteIntentParams,
		sourceChain: string,
		destinationChain: string,
	): Promise<QuoteIntentResult | PessimisticQuoteIntentResult>
	async quoteIntent(
		params: QuoteIntentParams,
		sourceChain: string,
		destinationChain: string,
	): Promise<QuoteIntentResult | PessimisticQuoteIntentResult> {
		const orderbook = this.orderbook()
		if (!params.optimistic) {
			const { quote, tokenIn, tokenOut } = await this.priceIntent(
				params,
				sourceChain,
				destinationChain,
				(route, amountIn) => orderbook.quotePessimistic(route, amountIn),
				levelQuoteTotals,
			)
			return {
				route: quote.route,
				side: quote.side,
				amountIn: fromOrderbookAmount(quote.amountIn, tokenIn.decimals),
				amountOut: fromOrderbookAmount(quote.amountOut, tokenOut.decimals),
				rate: quote.rate,
				priceBucket: quote.priceBucket,
				slippageBps: quote.slippageBps,
				fillable: quote.fillable,
				maxFillableIn: fromOrderbookAmount(quote.maxFillableIn, tokenIn.decimals),
			}
		}

		const { quote, tokenIn, tokenOut } = await this.priceIntent(
			params,
			sourceChain,
			destinationChain,
			(route, amountIn) => orderbook.quote(route, amountIn),
			swapQuoteTotals,
		)
		return {
			route: quote.route,
			side: quote.side,
			amountIn: fromOrderbookAmount(quote.amountIn, tokenIn.decimals),
			slippageBps: quote.slippageBps,
			fillable: quote.fillable,
			maxFillableIn: fromOrderbookAmount(quote.maxFillableIn, tokenIn.decimals),
			legs: quote.fills.map((fill) => ({
				advertisedSize: fromOrderbookAmount(fill.advertisedSize, tokenOut.decimals),
				orderRate: fill.orderRate,
				amountIn: fromOrderbookAmount(fill.amountIn, tokenIn.decimals),
				amountOut: fromOrderbookAmount(fill.amountOut, tokenOut.decimals),
			})),
		}
	}

	/**
	 * The fillable quote for an exact input, or for the input found to deliver an
	 * exact output, with the assets it trades.
	 */
	private async priceIntent<Q>(
		params: QuoteIntentParams,
		sourceChain: string,
		destinationChain: string,
		price: (route: OrderbookRoute, amountIn: bigint) => Promise<Q>,
		totals: (quote: Q) => QuoteTotals,
	): Promise<{ quote: Q; tokenIn: ResolvedAsset; tokenOut: ResolvedAsset }> {
		validateQuoteParams(params)
		const tokenIn = this.assetByAddress(sourceChain, params.tokenIn)
		const tokenOut = this.assetByAddress(destinationChain, params.tokenOut)
		const route = toRoute(tokenIn, tokenOut)

		if (params.amountIn !== undefined) {
			const quote = await price(route, toOrderbookAmount(params.amountIn, tokenIn.decimals))
			const { fillable, amountOut, maxFillableIn } = totals(quote)
			if (!fillable || fromOrderbookAmount(amountOut, tokenOut.decimals) === 0n) {
				throw this.insufficient(route, maxFillableIn, tokenIn)
			}
			return { quote, tokenIn, tokenOut }
		}

		const targetOut = toOrderbookAmount(params.amountOut as bigint, tokenOut.decimals)
		const quote = await this.quoteExactOutput(route, targetOut, tokenIn, price, totals)
		return { quote, tokenIn, tokenOut }
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
	 * guessing the input from the best rate, then keeping the part of each quote
	 * filled better than its worst rate and re-pricing the rest of `targetOut` at
	 * the input-to-output ratio that part filled at, until a quote delivers it.
	 * That ratio is the orderbook's own, with the protocol fee already taken off.
	 */
	private async quoteExactOutput<Q>(
		route: OrderbookRoute,
		targetOut: bigint,
		tokenIn: ResolvedAsset,
		price: (route: OrderbookRoute, amountIn: bigint) => Promise<Q>,
		totals: (quote: Q) => QuoteTotals,
	): Promise<Q> {
		const liquidity = await this.orderbook().routeLiquidity(route)
		if (liquidity.bestRate === null) throw this.insufficient(route, 0n, tokenIn)

		const inputUnit = toOrderbookAmount(1n, tokenIn.decimals)
		let amountIn = roundUpTo(requiredInput(liquidity.side, targetOut, liquidity.bestRate), inputUnit)
		let lastAmountIn = amountIn
		for (let round = 0; round < MAX_EXACT_OUTPUT_ROUNDS; round++) {
			// Past the most the route can fill, no input delivers the output.
			if (amountIn > liquidity.maxFillableIn) throw this.insufficient(route, liquidity.maxFillableIn, tokenIn)
			const served = await price(route, amountIn)
			const quote = totals(served)
			if (quote.fillable && quote.amountOut >= targetOut) return served
			lastAmountIn = amountIn
			const betterIn = quote.amountIn - quote.worstIn
			const betterOut = quote.amountOut - quote.worstOut
			const next =
				quote.fillable && quote.worstOut > 0n
					? betterIn + roundUpTo(divCeil((targetOut - betterOut) * quote.worstIn, quote.worstOut), inputUnit)
					: amountIn
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

function swapQuoteTotals(quote: OrderbookSwapQuote): QuoteTotals {
	// Fills come best price first.
	const worstFill = quote.fills.at(-1)
	if (quote.fillable && !worstFill)
		throw new OrderbookRequestError("the orderbook served a fillable quote with no fills")
	return {
		side: quote.side,
		amountIn: quote.amountIn,
		amountOut: quote.fills.reduce((total, fill) => total + fill.amountOut, 0n),
		fillable: quote.fillable,
		maxFillableIn: quote.maxFillableIn,
		worstIn: worstFill?.amountIn ?? 0n,
		worstOut: worstFill?.amountOut ?? 0n,
	}
}

function levelQuoteTotals(quote: OrderbookLevelQuote): QuoteTotals {
	if (quote.fillable && quote.rate === null) {
		throw new OrderbookRequestError("the orderbook served a fillable quote with no rate")
	}
	return {
		side: quote.side,
		amountIn: quote.amountIn,
		amountOut: quote.amountOut,
		fillable: quote.fillable,
		maxFillableIn: quote.maxFillableIn,
		worstIn: quote.amountIn,
		worstOut: quote.amountOut,
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
