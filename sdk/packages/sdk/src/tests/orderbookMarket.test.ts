import { strict as assert } from "node:assert"
import type { GraphQLClient } from "graphql-request"
import { parseUnits } from "viem"
import { ChainConfigService } from "@/configs/ChainConfigService"
import {
	HyperFxOrderbook,
	ORDERBOOK_QUERIES,
	ORDERBOOK_URLS,
	OrderbookRequestError,
	orderbookUrlFor,
} from "@/protocols/intents/orderbook/client"
import { OrderbookMarket } from "@/protocols/intents/orderbook/market"
import { OrderbookQuoteNotConvergedError, type QuoteIntentResult } from "@/protocols/intents/orderbook/types"

const CHAPEL = "EVM-97"
const AMOY = "EVM-80002"
const E18 = 10n ** 18n
const BOOKS = [{ id: "USDC-cNGN", base: "USDC", quote: "cNGN" }]

const config = new ChainConfigService()
const chapelUsdc = config.getUsdcAsset(CHAPEL) // 18 decimals
const chapelCngn = config.getCNgnAsset(CHAPEL)! // 6 decimals
const amoyUsdc = config.getUsdcAsset(AMOY) // 18 decimals
const amoyCngn = config.getCNgnAsset(AMOY)! // 6 decimals

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

interface Level {
	/** Quote per 1 base, before the protocol fee. */
	rate: bigint
	/** The tokenIn the level takes. */
	depth: bigint
}

/**
 * One side of the USDC/cNGN book as the orderbook serves it: `quote` fills the levels best first,
 * each at its own price, and `quotePessimistic` prices the whole input at the first level deep
 * enough for it, or the worst level. Every output has `slippageBps` taken off and is floored to a
 * whole raw unit of the output token. Amounts at 1e18.
 */
function side(
	kind: "BID" | "ASK",
	levels: Level[],
	opts: { slippageBps?: number; outDecimals: number; depthOut: bigint; availableLiquidity: bigint },
) {
	const slippageBps = opts.slippageBps ?? 0
	const unit = 10n ** BigInt(18 - opts.outDecimals)
	const maxFillableIn = levels.reduce((total, level) => total + level.depth, 0n)
	const grossOut = (amountIn: bigint, rate: bigint) =>
		kind === "BID" ? (amountIn * rate) / E18 : (amountIn * E18) / rate
	const out = (amountIn: bigint, rate: bigint) =>
		((grossOut(amountIn, rate) * BigInt(10_000 - slippageBps)) / 10_000n / unit) * unit
	return (query: string, variables: Record<string, any>) => {
		if (query === ORDERBOOK_QUERIES.routeLiquidity) {
			return {
				books: BOOKS,
				routeLiquidity: {
					route: "CROSS_CHAIN",
					bestRate: levels[0].rate.toString(),
					slippageBps,
					depthIn: maxFillableIn.toString(),
					depthOut: opts.depthOut.toString(),
					availableLiquidity: opts.availableLiquidity.toString(),
					maxFillableIn: maxFillableIn.toString(),
					orderCount: 3,
					solverCount: 2,
				},
			}
		}
		const amountIn = BigInt(variables.amountIn)
		const fillable = amountIn <= maxFillableIn
		if (query === ORDERBOOK_QUERIES.quotePessimistic) {
			const level = levels.find((l) => l.depth >= amountIn) ?? levels[levels.length - 1]
			return {
				quotePessimistic: {
					route: "CROSS_CHAIN",
					side: kind,
					amountIn: amountIn.toString(),
					amountOut: fillable ? out(amountIn, level.rate).toString() : "0",
					rate: fillable ? level.rate.toString() : null,
					priceBucket: fillable ? level.rate.toString() : null,
					slippageBps,
					fillable,
					maxFillableIn: maxFillableIn.toString(),
				},
			}
		}
		const fills = []
		let remaining = amountIn
		for (const level of levels) {
			if (!fillable || remaining === 0n) break
			const taken = remaining < level.depth ? remaining : level.depth
			remaining -= taken
			fills.push({
				advertisedSize: grossOut(level.depth, level.rate).toString(),
				orderRate: level.rate.toString(),
				amountIn: taken.toString(),
				amountOut: out(taken, level.rate).toString(),
			})
		}
		return {
			quote: {
				route: "CROSS_CHAIN",
				side: kind,
				amountIn: amountIn.toString(),
				slippageBps,
				fillable,
				maxFillableIn: maxFillableIn.toString(),
				fills,
			},
		}
	}
}

/**
 * A USDC/cNGN bid side: 1,000 USDC at 1,500 cNGN, then 4,000 more at 1,400, so nothing past
 * 5,000 USDC fills.
 */
function bidSide(slippageBps = 0) {
	return side(
		"BID",
		[
			{ rate: 1_500n * E18, depth: 1_000n * E18 },
			{ rate: 1_400n * E18, depth: 4_000n * E18 },
		],
		{ slippageBps, outDecimals: 6, depthOut: 7_000_000n * E18, availableLiquidity: 6_000_000n * E18 },
	)
}

/** A USDC/cNGN ask side at 1,500 cNGN per USDC, taking cNGN (6 decimals) for USDC (18). */
const askSide = side("ASK", [{ rate: 1_500n * E18, depth: 7_500_000n * E18 }], {
	outDecimals: 18,
	depthOut: 5_000n * E18,
	availableLiquidity: 5_000n * E18,
})

/**
 * An exact output's input is priced from the orderbook's outputs, which are floored to a raw
 * unit of cNGN, so it may exceed the least input that delivers by under one raw cNGN at 1,400.
 */
function assertNearMinimal(amountIn: bigint, minimal: bigint) {
	assert.ok(amountIn >= minimal && amountIn - minimal < 10n ** 12n / 1_400n, `${amountIn} is not near ${minimal}`)
}

const totalOut = (quote: QuoteIntentResult) => quote.legs.reduce((total, leg) => total + leg.amountOut, 0n)

const quotes = (calls: { query: string }[], query: string) => calls.filter((call) => call.query === query).length

describe("OrderbookMarket.quoteIntent optimistic", () => {
	it("quotes an exact input on the route by symbol and chain, at 1e18", async () => {
		const { market, calls } = stubOrderbook(bidSide())
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("100", 18), optimistic: true },
			CHAPEL,
			AMOY,
		)

		assert.equal(calls.length, 1)
		assert.equal(calls[0].query, ORDERBOOK_QUERIES.quote)
		assert.deepEqual(calls[0].variables.route, {
			tokenIn: "USDC",
			tokenOut: "cNGN",
			sourceChain: CHAPEL,
			destinationChain: AMOY,
		})
		assert.equal(calls[0].variables.amountIn, (100n * E18).toString())
		assert.deepEqual(quote, {
			route: "CROSS_CHAIN",
			side: "BID",
			amountIn: parseUnits("100", 18),
			slippageBps: 0,
			fillable: true,
			maxFillableIn: parseUnits("5000", 18),
			legs: [
				{
					advertisedSize: parseUnits("1500000", 6),
					orderRate: 1_500n * E18,
					amountIn: parseUnits("100", 18),
					amountOut: parseUnits("150000", 6),
				},
			],
		})
	})

	it("returns one leg per order, each at its own price", async () => {
		const { market } = stubOrderbook(bidSide())
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("2000", 18), optimistic: true },
			CHAPEL,
			AMOY,
		)
		// 1,000 USDC at 1,500 and 1,000 at 1,400.
		assert.deepEqual(
			quote.legs.map((leg) => [leg.orderRate, leg.amountIn, leg.amountOut]),
			[
				[1_500n * E18, parseUnits("1000", 18), parseUnits("1500000", 6)],
				[1_400n * E18, parseUnits("1000", 18), parseUnits("1400000", 6)],
			],
		)
	})

	it("scales a 6-decimal cNGN input up to 1e18", async () => {
		const { market, calls } = stubOrderbook(askSide)
		const quote = await market.quoteIntent(
			{ tokenIn: chapelCngn, tokenOut: amoyUsdc, amountIn: parseUnits("3000", 6), optimistic: true },
			CHAPEL,
			AMOY,
		)
		assert.equal(calls[0].variables.amountIn, parseUnits("3000", 18).toString())
		assert.equal(quote.amountIn, parseUnits("3000", 6))
		assert.equal(quote.legs[0].amountIn, parseUnits("3000", 6))
		assert.equal(quote.legs[0].amountOut, parseUnits("2", 18))
		assert.equal(quote.side, "ASK")
	})

	it("re-prices an exact output's worst leg until the legs deliver it", async () => {
		const { market, calls } = stubOrderbook(bidSide())
		const amountOut = parseUnits("2800000", 6)
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountOut, optimistic: true },
			CHAPEL,
			AMOY,
		)

		// 1,000 USDC at 1,500 delivers 1,500,000 cNGN; the other 1,300,000 needs 928.57… at 1,400.
		assert.equal(totalOut(quote), amountOut)
		assertNearMinimal(quote.amountIn, 1_928_571_428_571_428_571_429n)
		assert.equal(quote.legs.at(-1)?.orderRate, 1_400n * E18)
		assert.equal(quotes(calls, ORDERBOOK_QUERIES.quote), 2)
	})

	it("prices an exact output from the orderbook's outputs, which already have the protocol fee off", async () => {
		const { market } = stubOrderbook(bidSide(30))
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountOut: parseUnits("149550", 6), optimistic: true },
			CHAPEL,
			AMOY,
		)
		// 149,550 cNGN is 150,000 less 30 bps: 100 USDC at 1,500.
		assert.equal(quote.amountIn, parseUnits("100", 18))
		assert.equal(totalOut(quote), parseUnits("149550", 6))
		assert.equal(quote.slippageBps, 30)
	})

	it("rounds an exact output's input up to a whole raw unit of the input token", async () => {
		const { market } = stubOrderbook(askSide)
		const quote = await market.quoteIntent(
			{ tokenIn: chapelCngn, tokenOut: amoyUsdc, amountOut: 333_333_333_333_333_333n, optimistic: true },
			CHAPEL,
			AMOY,
		)
		// 0.333… USDC at 1,500 needs 499.9999999999999995 cNGN, rounded up to 6 decimals.
		assert.equal(quote.amountIn, parseUnits("500", 6))
	})

	it("returns an exact input the route cannot fill as unfillable", async () => {
		const { market } = stubOrderbook(bidSide())
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("6000", 18), optimistic: true },
			CHAPEL,
			AMOY,
		)
		assert.deepEqual(quote, {
			route: "CROSS_CHAIN",
			side: "BID",
			amountIn: parseUnits("6000", 18),
			slippageBps: 0,
			fillable: false,
			maxFillableIn: parseUnits("5000", 18),
			legs: [],
		})
	})

	it("returns an exact output past the route's depth as the orderbook's unfillable quote", async () => {
		const { market, calls } = stubOrderbook(bidSide())
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountOut: parseUnits("9000000", 6), optimistic: true },
			CHAPEL,
			AMOY,
		)
		// 9,000,000 cNGN at the best rate (1,500) needs 6,000 USDC, past the 5,000 the route fills.
		assert.equal(quote.fillable, false)
		assert.equal(quote.amountIn, parseUnits("6000", 18))
		assert.equal(quote.maxFillableIn, parseUnits("5000", 18))
		assert.deepEqual(quote.legs, [])
		assert.equal(quotes(calls, ORDERBOOK_QUERIES.quote), 1)
	})

	it("returns an exact output on a route no order serves as unfillable", async () => {
		const { market } = stubOrderbook((query, variables) => {
			if (query === ORDERBOOK_QUERIES.routeLiquidity) {
				return {
					books: BOOKS,
					routeLiquidity: {
						route: "CROSS_CHAIN",
						bestRate: null,
						slippageBps: 0,
						depthIn: "0",
						depthOut: "0",
						availableLiquidity: "0",
						maxFillableIn: "0",
						orderCount: 0,
						solverCount: 0,
					},
				}
			}
			return {
				quotePessimistic: {
					route: "CROSS_CHAIN",
					side: "BID",
					amountIn: variables.amountIn,
					amountOut: "0",
					rate: null,
					priceBucket: null,
					slippageBps: 0,
					fillable: false,
					maxFillableIn: "0",
				},
			}
		})
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountOut: parseUnits("1000", 6) },
			CHAPEL,
			AMOY,
		)
		assert.equal(quote.fillable, false)
		assert.equal(quote.amountOut, 0n)
		assert.equal(quote.maxFillableIn, 0n)
	})

	it("reports an exact output it cannot converge on apart from insufficient liquidity", async () => {
		// Deep enough, but no quote delivers more than one raw unit less than asked, so no round settles.
		const bids = bidSide()
		const cap = 1_000n * E18 - 10n ** 12n
		const { market, calls } = stubOrderbook((query, variables) => {
			const answer = bids(query, variables) as { quote?: { fills: { amountOut: string }[] } }
			for (const fill of answer.quote?.fills ?? [])
				if (BigInt(fill.amountOut) > cap) fill.amountOut = cap.toString()
			return answer
		})
		await assert.rejects(
			market.quoteIntent(
				{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountOut: parseUnits("1000", 6), optimistic: true },
				CHAPEL,
				AMOY,
			),
			(error: unknown) => error instanceof OrderbookQuoteNotConvergedError && error.rounds === 16,
		)
		assert.equal(quotes(calls, ORDERBOOK_QUERIES.quote), 16)
	})

	it("refuses a fillable quote the orderbook served with no fills", async () => {
		const bids = bidSide()
		const { market } = stubOrderbook((query, variables) => {
			const answer = bids(query, variables) as { quote: { fills: unknown[] } }
			answer.quote.fills = []
			return answer
		})
		await assert.rejects(
			market.quoteIntent(
				{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("100", 18), optimistic: true },
				CHAPEL,
				AMOY,
			),
			(error: unknown) => error instanceof OrderbookRequestError && /no fills/.test(error.message),
		)
	})

	it("requires exactly one amount", async () => {
		const { market } = stubOrderbook(bidSide())
		await assert.rejects(
			market.quoteIntent({ tokenIn: chapelUsdc, tokenOut: amoyCngn, optimistic: true }, CHAPEL, AMOY),
			/exactly one/,
		)
	})

	it("surfaces an orderbook GraphQL error", async () => {
		const { market } = stubOrderbook(() => {
			throw Object.assign(new Error("request failed"), {
				response: { errors: [{ message: "no book trades USDC for cNGN" }] },
			})
		})
		await assert.rejects(
			market.quoteIntent(
				{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: 1n, optimistic: true },
				CHAPEL,
				AMOY,
			),
			(error: unknown) => error instanceof OrderbookRequestError && /no book trades/.test(error.message),
		)
	})
})

describe("OrderbookMarket.quoteIntent pessimistic (default)", () => {
	it("prices the whole input at the first level deep enough for it", async () => {
		const { market, calls } = stubOrderbook(bidSide(30))
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("2000", 18) },
			CHAPEL,
			AMOY,
		)

		assert.equal(calls.length, 1)
		assert.equal(calls[0].query, ORDERBOOK_QUERIES.quotePessimistic)
		assert.equal(calls[0].variables.amountIn, (2_000n * E18).toString())
		// All 2,000 USDC at 1,400, less 30 bps.
		assert.deepEqual(quote, {
			route: "CROSS_CHAIN",
			side: "BID",
			amountIn: parseUnits("2000", 18),
			amountOut: parseUnits("2791600", 6),
			rate: 1_400n * E18,
			priceBucket: 1_400n * E18,
			slippageBps: 30,
			fillable: true,
			maxFillableIn: parseUnits("5000", 18),
		})
	})

	it("raises an exact output's input at each quote's level price until it delivers", async () => {
		const { market, calls } = stubOrderbook(bidSide())
		const amountOut = parseUnits("2800000", 6)
		const quote = await market.quoteIntent({ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountOut }, CHAPEL, AMOY)

		// 2,800,000 cNGN at the best rate (1,500) needs 1,866.67 USDC, which prices at 1,400:
		// the second round asks for about 2,000 USDC, which delivers 2,800,000.
		assert.equal(quote.amountOut, amountOut)
		assertNearMinimal(quote.amountIn, parseUnits("2000", 18))
		assert.equal(quote.rate, 1_400n * E18)
		assert.equal(quotes(calls, ORDERBOOK_QUERIES.quotePessimistic), 2)
	})

	it("returns an exact input the route cannot fill as unfillable", async () => {
		const { market } = stubOrderbook(bidSide())
		const quote = await market.quoteIntent(
			{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("6000", 18) },
			CHAPEL,
			AMOY,
		)
		assert.deepEqual(quote, {
			route: "CROSS_CHAIN",
			side: "BID",
			amountIn: parseUnits("6000", 18),
			amountOut: 0n,
			rate: null,
			priceBucket: null,
			slippageBps: 0,
			fillable: false,
			maxFillableIn: parseUnits("5000", 18),
		})
	})

	it("refuses a fillable quote the orderbook served with no rate", async () => {
		const bids = bidSide()
		const { market } = stubOrderbook((query, variables) => {
			const answer = bids(query, variables) as { quotePessimistic: { rate: string | null } }
			answer.quotePessimistic.rate = null
			return answer
		})
		await assert.rejects(
			market.quoteIntent(
				{ tokenIn: chapelUsdc, tokenOut: amoyCngn, amountIn: parseUnits("100", 18) },
				CHAPEL,
				AMOY,
			),
			(error: unknown) => error instanceof OrderbookRequestError && /no rate/.test(error.message),
		)
	})
})

describe("OrderbookMarket.availableLiquidity", () => {
	it("labels the route's side and units from the book", async () => {
		const { market } = stubOrderbook(bidSide())
		const liquidity = await market.availableLiquidity({ tokenIn: chapelUsdc, tokenOut: amoyCngn }, CHAPEL, AMOY)
		assert.deepEqual(liquidity, {
			sourceChain: CHAPEL,
			destinationChain: AMOY,
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cNGN",
			tokenAddress: amoyCngn,
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
			fillChain: AMOY,
			rate: value.toString(),
			depthIn: "0",
			depthOut: "0",
			backingLiquidity: "0",
			sourceChains: [CHAPEL],
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
			sourceChain: CHAPEL,
			destinationChain: AMOY,
			tokenInSymbol: "cngn",
			tokenOutSymbol: "usdc",
		})

		assert.deepEqual(calls[0].variables, { tokenA: "cNGN", tokenB: "USDC", fillChain: AMOY, sourceChain: CHAPEL })
		assert.deepEqual(rates, {
			baseTokenSymbol: "USDC",
			quoteTokenSymbol: "cNGN",
			sourceChain: CHAPEL,
			destinationChain: AMOY,
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
			sourceChain: AMOY,
			destinationChain: AMOY,
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cNGN",
		})
		assert.equal(crossedRates.spread, null)
		assert.equal(crossedRates.mid, "1605")

		const oneSided = stubOrderbook(() => ({ books: BOOKS, aToB: rate("BID", 1_580n * E18), bToA: null }))
		const oneSidedRates = await oneSided.market.buyAndSellRates({
			sourceChain: AMOY,
			destinationChain: AMOY,
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cNGN",
		})
		assert.equal(oneSidedRates.ask, null)
		assert.equal(oneSidedRates.mid, null)
		assert.equal(oneSidedRates.spreadBps, null)
	})
})

describe("HyperFxOrderbook symbol case", () => {
	it("finds the route's book and side whatever case the symbols are named in", async () => {
		const client = { request: async (query: string, variables: Record<string, any>) => bidSide()(query, variables) }
		const orderbook = new HyperFxOrderbook(client as unknown as GraphQLClient)
		const liquidity = await orderbook.routeLiquidity({
			tokenIn: "usdc",
			tokenOut: "CNGN",
			sourceChain: CHAPEL,
			destinationChain: AMOY,
		})
		assert.deepEqual(liquidity.book, BOOKS[0])
		assert.equal(liquidity.side, "BID")
	})
})

describe("orderbookUrlFor", () => {
	it("serves testnet chains from the testnet deployment and everything else from mainnet", () => {
		assert.equal(orderbookUrlFor(CHAPEL), ORDERBOOK_URLS.testnet)
		assert.equal(orderbookUrlFor(AMOY), ORDERBOOK_URLS.testnet)
		assert.equal(orderbookUrlFor("EVM-8453"), ORDERBOOK_URLS.mainnet)
		assert.equal(orderbookUrlFor("EVM-56"), ORDERBOOK_URLS.mainnet)
	})
})
