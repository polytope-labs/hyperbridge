import { GraphQLClient } from "graphql-request"
import { TESTNET_CHAINS } from "@/utils"

/** The HyperFX orderbook deployments, one per network. */
export const ORDERBOOK_URLS = {
	mainnet: "https://orderbook.hyperfx.finance/mainnet/graphql",
	testnet: "https://orderbook.hyperfx.finance/testnet/graphql",
} as const

/** The orderbook deployment that serves `stateMachineId`: the testnet book for a testnet chain, else mainnet. */
export function orderbookUrlFor(stateMachineId: string): string {
	return TESTNET_CHAINS.has(stateMachineId) ? ORDERBOOK_URLS.testnet : ORDERBOOK_URLS.mainnet
}

/** Every amount and rate the orderbook takes or returns is fixed-point at 1e18, whatever the token's decimals. */
export const ORDERBOOK_DECIMALS = 18
export const ORDERBOOK_SCALE = 10n ** BigInt(ORDERBOOK_DECIMALS)

/** `BID` takes a swap selling the book's base; `ASK` takes a swap buying it. */
export type OrderbookSide = "BID" | "ASK"

/** `SAME_CHAIN` is served by every order on the chain; `CROSS_CHAIN` only by orders declaring the source. */
export type OrderbookRouteKind = "SAME_CHAIN" | "CROSS_CHAIN"

/** A swap route, by token symbol and state machine id (`EVM-8453`). */
export interface OrderbookRoute {
	tokenIn: string
	tokenOut: string
	/** Where the swapper's `tokenIn` is escrowed. */
	sourceChain: string
	/** Where `tokenOut` is delivered, which is the chain the order is filled on. */
	destinationChain: string
}

/** A configured book. Every rate on it is `quote` per 1 `base`. */
export interface OrderbookBook {
	id: string
	base: string
	quote: string
}

/** The best price level for one direction on one fill chain. Amounts and rates at 1e18. */
export interface OrderbookRate {
	side: OrderbookSide
	fillChain: string
	/** The level's best single-order price, quote per 1 base. */
	rate: bigint
	/** Virtual depth: the level's summed order sizes, in tokenIn and tokenOut. */
	depthIn: bigint
	depthOut: bigint
	/** True liquidity: each solver's largest validated order in the level, summed, in tokenOut. */
	backingLiquidity: bigint
	sourceChains: string[]
	orderCount: number
	solverCount: number
}

/** What a route holds, without an amount. Amounts and rates at 1e18. */
export interface OrderbookRouteLiquidity {
	route: OrderbookRouteKind
	/** The book the route trades on, which names the units of `bestRate`. */
	book: OrderbookBook
	/** `BID` when tokenIn is the book's base, `ASK` when it is the quote. */
	side: OrderbookSide
	/** Quote per 1 base, before the protocol fee; null when no order serves the route. */
	bestRate: bigint | null
	/** The destination chain's protocol fee in basis points, which the gateway takes out of what the orders deliver. */
	slippageBps: number
	/** Virtual depth: the serving orders' sizes summed, in tokenIn and tokenOut. */
	depthIn: bigint
	depthOut: bigint
	/** True liquidity in tokenOut: each serving solver's largest validated order, summed. */
	availableLiquidity: bigint
	/** The largest tokenIn amount the route could fill with its orders combined. */
	maxFillableIn: bigint
	orderCount: number
	solverCount: number
}

/** One order an optimistic quote takes, at the order's own price. Amounts and rates at 1e18. */
export interface OrderbookQuoteFill {
	/** The order's full advertised size, not just the part this fill takes. */
	advertisedSize: bigint
	/** The order's own price, quote per 1 base, before the protocol fee: what this fill settles at. */
	orderRate: bigint
	/** The tokenIn this fill takes, in whole raw units on the source chain. */
	amountIn: bigint
	/** The tokenOut this fill delivers, after the protocol fee, floored to a whole raw unit on the destination chain. */
	amountOut: bigint
}

/**
 * An optimistic quote for an input amount on a route: the trade split across the route's orders,
 * best price first, each at its own price. There is no one rate and no total output; the fills are
 * the quote. Amounts and rates at 1e18.
 */
export interface OrderbookSwapQuote {
	route: OrderbookRouteKind
	side: OrderbookSide
	/** The tokenIn priced, floored to a whole raw unit on the source chain. */
	amountIn: bigint
	/** The destination chain's protocol fee in basis points, already taken off every fill's `amountOut`. */
	slippageBps: number
	fillable: boolean
	/** The largest tokenIn amount the route's orders could take together, each at its own price. */
	maxFillableIn: bigint
	/** The orders used, best price first; empty when not fillable. */
	fills: OrderbookQuoteFill[]
}

/**
 * A pessimistic quote for an input amount on a route: one price, from the first level, best first,
 * deep enough to fill the whole trade by itself, or else the route's worst price when its levels
 * together can. Amounts and rates at 1e18.
 */
export interface OrderbookLevelQuote {
	route: OrderbookRouteKind
	side: OrderbookSide
	/** The tokenIn priced, floored to a whole raw unit on the source chain. */
	amountIn: bigint
	/** The tokenOut delivered at `rate`, after the protocol fee, floored to a whole raw unit on the destination chain; 0 when not fillable. */
	amountOut: bigint
	/** The worst single-order price in the level that fills it, quote per 1 base, before the protocol fee; null when not fillable. */
	rate: bigint | null
	/** The price bucket of that level; null when not fillable. */
	priceBucket: bigint | null
	/** The destination chain's protocol fee in basis points, already taken off `amountOut`. */
	slippageBps: number
	fillable: boolean
	/** The largest tokenIn amount this quote could fill: any one level at its worst price, or the whole route at its worst. */
	maxFillableIn: bigint
}

/** The best bid and ask a route can reach, with the book that orients them. */
export interface OrderbookTopOfBook {
	book: OrderbookBook
	/** Highest bid: the most quote a seller of 1 base receives. */
	bid: OrderbookRate | null
	/** Lowest ask: the least quote a buyer of 1 base pays. */
	ask: OrderbookRate | null
}

/** The orderbook could not answer: a transport failure, or a GraphQL error such as a pair no book trades. */
export class OrderbookRequestError extends Error {
	constructor(message: string) {
		super(`HyperFX orderbook request failed: ${message}`)
		this.name = "OrderbookRequestError"
	}
}

const RATE_FIELDS = "side fillChain rate depthIn depthOut backingLiquidity sourceChains orderCount solverCount"

const TOP_OF_BOOK_QUERY = `
query TopOfBook($tokenA: String!, $tokenB: String!, $fillChain: String!, $sourceChain: String!) {
  books { id base quote }
  aToB: bestRate(tokenIn: $tokenA, tokenOut: $tokenB, fillChain: $fillChain, sourceChain: $sourceChain) { ${RATE_FIELDS} }
  bToA: bestRate(tokenIn: $tokenB, tokenOut: $tokenA, fillChain: $fillChain, sourceChain: $sourceChain) { ${RATE_FIELDS} }
}`

const ROUTE_LIQUIDITY_QUERY = `
query RouteLiquidity($route: RouteInput!) {
  books { id base quote }
  routeLiquidity(route: $route) {
    route bestRate slippageBps depthIn depthOut availableLiquidity maxFillableIn orderCount solverCount
  }
}`

const QUOTE_QUERY = `
query Quote($route: RouteInput!, $amountIn: BigInt!) {
  quote(route: $route, amountIn: $amountIn) {
    route side amountIn slippageBps fillable maxFillableIn
    fills { advertisedSize orderRate amountIn amountOut }
  }
}`

const QUOTE_PESSIMISTIC_QUERY = `
query QuotePessimistic($route: RouteInput!, $amountIn: BigInt!) {
  quotePessimistic(route: $route, amountIn: $amountIn) {
    route side amountIn amountOut rate priceBucket slippageBps fillable maxFillableIn
  }
}`

/** Every document the client sends, so they can be checked against the orderbook's published schema. */
export const ORDERBOOK_QUERIES = {
	topOfBook: TOP_OF_BOOK_QUERY,
	routeLiquidity: ROUTE_LIQUIDITY_QUERY,
	quote: QUOTE_QUERY,
	quotePessimistic: QUOTE_PESSIMISTIC_QUERY,
} as const

/**
 * Read-only client for the HyperFX orderbook's GraphQL API.
 *
 * Tokens are named by symbol and chains by state machine id. Every amount and
 * rate is a `bigint` at 1e18, as the orderbook serves it; rates are quote per 1
 * base of the pair's book.
 */
export class HyperFxOrderbook {
	private readonly client: GraphQLClient

	/** @param urlOrClient - The orderbook's GraphQL URL, or a configured `graphql-request` client. */
	constructor(urlOrClient: string | GraphQLClient = ORDERBOOK_URLS.mainnet) {
		this.client = typeof urlOrClient === "string" ? new GraphQLClient(urlOrClient) : urlOrClient
	}

	/**
	 * The best bid and ask on `fillChain` for the pair `tokenA`/`tokenB`, counting
	 * only orders a swap from `sourceChain` can use.
	 */
	async topOfBook(params: {
		tokenA: string
		tokenB: string
		fillChain: string
		sourceChain: string
	}): Promise<OrderbookTopOfBook> {
		const data = await this.request<{
			books: OrderbookBook[]
			aToB: RawRate | null
			bToA: RawRate | null
		}>(TOP_OF_BOOK_QUERY, params)
		const book = findBook(data.books, params.tokenA, params.tokenB)
		const rates = [data.aToB, data.bToA].filter((rate): rate is RawRate => rate !== null).map(parseRate)
		return {
			book,
			bid: rates.find((rate) => rate.side === "BID") ?? null,
			ask: rates.find((rate) => rate.side === "ASK") ?? null,
		}
	}

	/** Total liquidity on a route, no amount required. */
	async routeLiquidity(route: OrderbookRoute): Promise<OrderbookRouteLiquidity> {
		const { books, routeLiquidity: raw } = await this.request<{
			books: OrderbookBook[]
			routeLiquidity: RawRouteLiquidity
		}>(ROUTE_LIQUIDITY_QUERY, { route })
		const book = findBook(books, route.tokenIn, route.tokenOut)
		return {
			route: raw.route,
			book,
			side: sameSymbol(book.base, route.tokenIn) ? "BID" : "ASK",
			bestRate: optionalBigInt(raw.bestRate),
			slippageBps: raw.slippageBps,
			depthIn: BigInt(raw.depthIn),
			depthOut: BigInt(raw.depthOut),
			availableLiquidity: BigInt(raw.availableLiquidity),
			maxFillableIn: BigInt(raw.maxFillableIn),
			orderCount: raw.orderCount,
			solverCount: raw.solverCount,
		}
	}

	/**
	 * The optimistic quote for `amountIn` (1e18 tokenIn) on a route: the route's orders, best price
	 * first, each taking what it can at its own price. The fills' `amountOut`s sum to the output.
	 */
	async quote(route: OrderbookRoute, amountIn: bigint): Promise<OrderbookSwapQuote> {
		const { quote: raw } = await this.request<{ quote: RawSwapQuote }>(QUOTE_QUERY, {
			route,
			amountIn: amountIn.toString(),
		})
		return {
			route: raw.route,
			side: raw.side,
			amountIn: BigInt(raw.amountIn),
			slippageBps: raw.slippageBps,
			fillable: raw.fillable,
			maxFillableIn: BigInt(raw.maxFillableIn),
			fills: raw.fills.map((fill) => ({
				advertisedSize: BigInt(fill.advertisedSize),
				orderRate: BigInt(fill.orderRate),
				amountIn: BigInt(fill.amountIn),
				amountOut: BigInt(fill.amountOut),
			})),
		}
	}

	/**
	 * The pessimistic quote for `amountIn` (1e18 tokenIn) on a route: one price, from the first level
	 * deep enough to fill the whole trade by itself, at which every order in it fills; the route's
	 * worst price when no one level can but its levels together can.
	 */
	async quotePessimistic(route: OrderbookRoute, amountIn: bigint): Promise<OrderbookLevelQuote> {
		const { quotePessimistic: raw } = await this.request<{ quotePessimistic: RawLevelQuote }>(
			QUOTE_PESSIMISTIC_QUERY,
			{ route, amountIn: amountIn.toString() },
		)
		return {
			route: raw.route,
			side: raw.side,
			amountIn: BigInt(raw.amountIn),
			amountOut: BigInt(raw.amountOut),
			rate: optionalBigInt(raw.rate),
			priceBucket: optionalBigInt(raw.priceBucket),
			slippageBps: raw.slippageBps,
			fillable: raw.fillable,
			maxFillableIn: BigInt(raw.maxFillableIn),
		}
	}

	private async request<T>(query: string, variables: Record<string, unknown>): Promise<T> {
		try {
			return await this.client.request<T>(query, variables)
		} catch (error) {
			throw new OrderbookRequestError(describeError(error))
		}
	}
}

function describeError(error: unknown): string {
	const graphqlErrors = (error as { response?: { errors?: { message: string }[] } })?.response?.errors
	if (graphqlErrors?.length) return graphqlErrors.map((e) => e.message).join("; ")
	return error instanceof Error ? error.message : String(error)
}

/** The orderbook matches symbols without regard to case, and answers in its configured spelling. */
function sameSymbol(a: string, b: string): boolean {
	return a.toLowerCase() === b.toLowerCase()
}

function findBook(books: OrderbookBook[], tokenA: string, tokenB: string): OrderbookBook {
	const book = books.find(
		(b) =>
			(sameSymbol(b.base, tokenA) && sameSymbol(b.quote, tokenB)) ||
			(sameSymbol(b.base, tokenB) && sameSymbol(b.quote, tokenA)),
	)
	if (!book) throw new OrderbookRequestError(`no book trades ${tokenA} for ${tokenB}`)
	return book
}

function optionalBigInt(value: string | null): bigint | null {
	return value === null ? null : BigInt(value)
}

function parseRate(raw: RawRate): OrderbookRate {
	return {
		side: raw.side,
		fillChain: raw.fillChain,
		rate: BigInt(raw.rate),
		depthIn: BigInt(raw.depthIn),
		depthOut: BigInt(raw.depthOut),
		backingLiquidity: BigInt(raw.backingLiquidity),
		sourceChains: raw.sourceChains,
		orderCount: raw.orderCount,
		solverCount: raw.solverCount,
	}
}

interface RawRate {
	side: OrderbookSide
	fillChain: string
	rate: string
	depthIn: string
	depthOut: string
	backingLiquidity: string
	sourceChains: string[]
	orderCount: number
	solverCount: number
}

interface RawRouteLiquidity {
	route: OrderbookRouteKind
	bestRate: string | null
	slippageBps: number
	depthIn: string
	depthOut: string
	availableLiquidity: string
	maxFillableIn: string
	orderCount: number
	solverCount: number
}

interface RawSwapQuote {
	route: OrderbookRouteKind
	side: OrderbookSide
	amountIn: string
	slippageBps: number
	fillable: boolean
	maxFillableIn: string
	fills: { advertisedSize: string; orderRate: string; amountIn: string; amountOut: string }[]
}

interface RawLevelQuote {
	route: OrderbookRouteKind
	side: OrderbookSide
	amountIn: string
	amountOut: string
	rate: string | null
	priceBucket: string | null
	slippageBps: number
	fillable: boolean
	maxFillableIn: string
}
