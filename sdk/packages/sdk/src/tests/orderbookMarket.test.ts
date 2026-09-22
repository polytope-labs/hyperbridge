import { strict as assert } from "node:assert"
import type { GraphQLClient } from "graphql-request"
import { parseUnits } from "viem"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { HyperFxOrderbook, ORDERBOOK_QUERIES, OrderbookRequestError } from "@/protocols/intents/orderbook/client"
import { OrderbookMarket } from "@/protocols/intents/orderbook/market"
import { InsufficientOrderbookLiquidityError } from "@/protocols/intents/orderbook/types"

const BSC = "EVM-56"
const BASE = "EVM-8453"
const E18 = 10n ** 18n
const BOOKS = [{ id: "USDC-cNGN", base: "USDC", quote: "cNGN" }]

const config = new ChainConfigService()
const bscUsdc = config.getUsdcAsset(BSC) // 18 decimals
const baseUsdc = config.getUsdcAsset(BASE) // 6 decimals
const baseCngn = config.getCNgnAsset(BASE)! // 6 decimals

type Handler = (query: string, variables: Record<string, any>) => unknown

/** An orderbook whose GraphQL transport answers from `handler` and records every request. */
function stubOrderbook(handler: Handler) {
	const calls: { query: string; variables: Record<string, any> }[] = []
	const client = {
		request: async (query: string, variables: Record<string, any>) => {
			calls.push({ query, variables })
			return handler(query, variables)
		},
	} as unknown as GraphQLClient
	const orderbook = new HyperFxOrderbook(client)
	return { market: new OrderbookMarket(config, () => orderbook), calls }
}

/**
 * A USDC/cNGN bid side: the first 1,000 USDC clears at 1,500 cNGN, anything
 * larger at 1,400, and nothing past 5,000 USDC fills. Amounts at 1e18.
 */
function bidSide(query: string, variables: Record<string, any>) {
	const maxFillableIn = 5_000n * E18
	if (query === ORDERBOOK_QUERIES.routeLiquidity) {
		return {
			books: BOOKS,
			routeLiquidity: {
				route: "CROSS_CHAIN",
				bestRate: (1_500n * E18).toString(),
				depthIn: maxFillableIn.toString(),
				depthOut: (7_000_000n * E18).toString(),
				availableLiquidity: (6_000_000n * E18).toString(),
				maxFillableIn: maxFillableIn.toString(),
				orderCount: 3,
				solverCount: 2,
			},
		}
	}
	const amountIn = BigInt(variables.amountIn)
	const fillable = amountIn <= maxFillableIn
	const rate = amountIn <= 1_000n * E18 ? 1_500n * E18 : 1_400n * E18
	// The orderbook floors amountOut to a whole raw unit of cNGN (6 decimals).
	const amountOut = fillable ? ((amountIn * rate) / E18 / 10n ** 12n) * 10n ** 12n : 0n
	return {
		quote: {
			route: "CROSS_CHAIN",
			side: "BID",
			amountIn: amountIn.toString(),
			amountOut: amountOut.toString(),
			rate: fillable ? rate.toString() : null,
			fillable,
			depth: amountIn.toString(),
			maxFillableIn: maxFillableIn.toString(),
			fills: fillable
				? [{ orderRate: rate.toString(), amountOut: amountOut.toString(), advertisedSize: "1" }]
				: [],
		},
	}
}

describe("OrderbookMarket.quoteIntent", () => {
	it("quotes an exact input on the route by symbol and chain, at 1e18", async () => {
		const { market, calls } = stubOrderbook(bidSide)
		const quote = await market.quoteIntent(
			{ tokenIn: bscUsdc, tokenOut: baseCngn, amountIn: parseUnits("100", 18) },
			BSC,
			BASE,
		)

		assert.equal(calls.length, 1)
		assert.deepEqual(calls[0].variables.route, {
			tokenIn: "USDC",
			tokenOut: "cNGN",
			sourceChain: BSC,
			destinationChain: BASE,
		})
		assert.equal(calls[0].variables.amountIn, (100n * E18).toString())
		assert.equal(quote.tradeType, "EXACT_INPUT")
		assert.equal(quote.amountIn, parseUnits("100", 18))
		assert.equal(quote.amountOut, parseUnits("150000", 6))
		assert.deepEqual(quote.quoteMetadata, {
			sourceChain: BSC,
			destinationChain: BASE,
			route: "CROSS_CHAIN",
			side: "BID",
			baseTokenSymbol: "USDC",
			quoteTokenSymbol: "cNGN",
			rate: "1500",
			maxFillableIn: parseUnits("5000", 18),
			orderCount: 1,
		})
	})

	it("scales a 6-decimal input up to 1e18", async () => {
		const { market, calls } = stubOrderbook(bidSide)
		await market.quoteIntent({ tokenIn: baseUsdc, tokenOut: baseCngn, amountIn: parseUnits("2.5", 6) }, BASE, BASE)
		assert.equal(calls[0].variables.amountIn, parseUnits("2.5", 18).toString())
	})

	it("raises an exact output's input until the clearing price delivers it", async () => {
		const { market, calls } = stubOrderbook(bidSide)
		const amountOut = parseUnits("2800000", 6)
		const quote = await market.quoteIntent({ tokenIn: bscUsdc, tokenOut: baseCngn, amountOut }, BSC, BASE)

		// 2,800,000 cNGN at the best rate (1,500) needs 1,866.67 USDC, which clears at 1,400:
		// the second round asks for 2,000 USDC, which delivers exactly 2,800,000.
		assert.equal(quote.tradeType, "EXACT_OUTPUT")
		assert.equal(quote.amountOut, amountOut)
		assert.equal(quote.amountIn, parseUnits("2000", 18))
		assert.equal(quote.quoteMetadata.rate, "1400")
		assert.equal(calls.filter((call) => call.query === ORDERBOOK_QUERIES.quote).length, 2)
	})

	it("rounds an exact output's input up to a whole raw unit of the input token", async () => {
		const { market } = stubOrderbook(bidSide)
		const quote = await market.quoteIntent(
			{ tokenIn: baseUsdc, tokenOut: baseCngn, amountOut: parseUnits("1000", 6) },
			BASE,
			BASE,
		)
		// 1,000 / 1,500 = 0.666666… USDC, rounded up to 6 decimals.
		assert.equal(quote.amountIn, 666_667n)
	})

	it("refuses an exact input the route cannot fill", async () => {
		const { market } = stubOrderbook(bidSide)
		await assert.rejects(
			market.quoteIntent({ tokenIn: bscUsdc, tokenOut: baseCngn, amountIn: parseUnits("6000", 18) }, BSC, BASE),
			(error: unknown) =>
				error instanceof InsufficientOrderbookLiquidityError && error.maxFillableIn === parseUnits("5000", 18),
		)
	})

	it("refuses an exact output past the route's depth without quoting it", async () => {
		const { market, calls } = stubOrderbook(bidSide)
		await assert.rejects(
			market.quoteIntent(
				{ tokenIn: bscUsdc, tokenOut: baseCngn, amountOut: parseUnits("9000000", 6) },
				BSC,
				BASE,
			),
			InsufficientOrderbookLiquidityError,
		)
		assert.equal(calls.filter((call) => call.query === ORDERBOOK_QUERIES.quote).length, 0)
	})

	it("requires exactly one amount", async () => {
		const { market } = stubOrderbook(bidSide)
		await assert.rejects(market.quoteIntent({ tokenIn: bscUsdc, tokenOut: baseCngn }, BSC, BASE), /exactly one/)
	})

	it("surfaces an orderbook GraphQL error", async () => {
		const { market } = stubOrderbook(() => {
			throw Object.assign(new Error("request failed"), {
				response: { errors: [{ message: "no book trades USDC for cNGN" }] },
			})
		})
		await assert.rejects(
			market.quoteIntent({ tokenIn: bscUsdc, tokenOut: baseCngn, amountIn: 1n }, BSC, BASE),
			(error: unknown) => error instanceof OrderbookRequestError && /no book trades/.test(error.message),
		)
	})
})

describe("OrderbookMarket.availableLiquidity", () => {
	it("labels the route's side and units from the book", async () => {
		const { market } = stubOrderbook(bidSide)
		const liquidity = await market.availableLiquidity({ tokenIn: bscUsdc, tokenOut: baseCngn }, BSC, BASE)
		assert.deepEqual(liquidity, {
			sourceChain: BSC,
			destinationChain: BASE,
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cNGN",
			tokenAddress: baseCngn,
			route: "CROSS_CHAIN",
			side: "BID",
			baseTokenSymbol: "USDC",
			quoteTokenSymbol: "cNGN",
			bestRate: "1500",
			availableLiquidity: "6000000",
			depthIn: "5000",
			depthOut: "7000000",
			maxFillableIn: "5000",
			orderCount: 3,
			solverCount: 2,
		})
	})
})

describe("OrderbookMarket.buyAndSellRates", () => {
	function rate(side: "BID" | "ASK", value: bigint) {
		return {
			side,
			fillChain: BASE,
			rate: value.toString(),
			depthIn: "0",
			depthOut: "0",
			backingLiquidity: "0",
			sourceChains: [BSC],
			orderCount: 1,
			solverCount: 1,
		}
	}

	it("orients the bid and ask on the book whichever way the pair is named", async () => {
		const { market, calls } = stubOrderbook(() => ({
			books: BOOKS,
			// Named cNGN first, so the first alias is the ask side.
			aToB: rate("ASK", 1_600n * E18),
			bToA: rate("BID", 1_580n * E18),
		}))
		const rates = await market.buyAndSellRates({
			sourceChain: BSC,
			destinationChain: BASE,
			tokenInSymbol: "cngn",
			tokenOutSymbol: "usdc",
		})

		assert.deepEqual(calls[0].variables, { tokenA: "cNGN", tokenB: "USDC", fillChain: BASE, sourceChain: BSC })
		assert.deepEqual(rates, {
			baseTokenSymbol: "USDC",
			quoteTokenSymbol: "cNGN",
			sourceChain: BSC,
			destinationChain: BASE,
			bid: "1580",
			ask: "1600",
			mid: "1590",
			spread: "20",
			spreadBps: 125.78,
		})
	})

	it("reports no spread for a crossed book or an empty side", async () => {
		const crossed = stubOrderbook(() => ({
			books: BOOKS,
			aToB: rate("BID", 1_610n * E18),
			bToA: rate("ASK", 1_600n * E18),
		}))
		const crossedRates = await crossed.market.buyAndSellRates({
			sourceChain: BASE,
			destinationChain: BASE,
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cNGN",
		})
		assert.equal(crossedRates.spread, null)
		assert.equal(crossedRates.mid, "1605")

		const oneSided = stubOrderbook(() => ({ books: BOOKS, aToB: rate("BID", 1_580n * E18), bToA: null }))
		const oneSidedRates = await oneSided.market.buyAndSellRates({
			sourceChain: BASE,
			destinationChain: BASE,
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cNGN",
		})
		assert.equal(oneSidedRates.ask, null)
		assert.equal(oneSidedRates.mid, null)
		assert.equal(oneSidedRates.spreadBps, null)
	})
})
