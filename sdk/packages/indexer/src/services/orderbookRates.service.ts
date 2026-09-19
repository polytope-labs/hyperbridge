// USD prices for tokens without a $1 peg, taken from the HyperFX orderbook's rates. Every rate is
// quote per 1 base, on both sides of the book, and a rate entry names the two symbols outright — so
// a token's USD price is its rate against a $1 stable, inverted when the stable is the base.
//
// On determinism: a rate describes now, so a resync prices a historical order at today's rate. The
// orderbook keeps no rate history to read at a past block, so this is inherent to the number rather
// than a shortcut, and it is why only the USD rollup depends on it: the raw per-token volume beside
// it is exact and is recorded whether or not a price is found.
import Decimal from "decimal.js"

import { GRAPHQL_PATH, ORDERBOOK_URL_VAR, orderbookEndpoint } from "@/utils/orderbook"
import { safeFetch } from "@/utils/safeFetch"

/**
 * Symbols taken to be worth exactly $1. They are also the quote currencies every other token is
 * priced against, in this order: the first one whose book quotes the token wins.
 */
export const STABLE_SYMBOLS = ["USDC", "USDT"]

/** A price is read with a handler blocked on it, so a slow orderbook must not hold one up for long. */
export const FETCH_TIMEOUT_MS = 5_000
/** How long the orderbook's answer — a rate, or "no book quotes this" — is reused for. */
export const PRICE_TTL_MS = 60_000
/** How long an unreachable orderbook is left alone before the next token priced retries it. */
export const FAILURE_TTL_MS = 5_000
/** Longer than any real ERC-20 symbol. A token whose `symbol()` returns junk is never queried. */
export const MAX_SYMBOL_LENGTH = 32
/** A missing URL is reported at most this often: every token priced would otherwise say it. */
export const SKIP_REPORT_INTERVAL_MS = 60_000

/** Rates are fixed-point at 1e18, quote per 1 base. */
const WAD = new Decimal(10).pow(18)

// One round trip prices the token against every stable. An alias for a pair no book trades is left
// out of `data` with an error beside it, and the aliases next to it still answer; `data` is null
// only when every alias failed, which is the ordinary shape for an unlisted token.
//
// `base` and `quote` are asked for rather than `side`: they name the rate's units directly, where
// `side` only says which of tokenIn and tokenOut the base is, leaving the units to be derived.
const QUERY = `query Rates($symbol: String!) { ${STABLE_SYMBOLS.map(
	(stable, index) => `q${index}: bestRate(tokenIn: $symbol, tokenOut: ${JSON.stringify(stable)}) { base quote rate }`,
).join(" ")} }`

interface Outcome {
	price: Decimal | null
	/** Whether the orderbook failed to answer, rather than answering that it has no rate. */
	failed: boolean
}

const NO_PRICE: Outcome = { price: null, failed: false }
const FAILED: Outcome = { price: null, failed: true }

interface CacheEntry {
	price: Promise<Decimal | null>
	/** Infinity until the fetch settles, so callers in the same block share it instead of racing. */
	expiresAt: number
}

const cache = new Map<string, CacheEntry>()

let lastSkipReport = 0

/** Forgets every cached rate and the skip report clock. For tests. */
export function resetOrderbookRates(): void {
	cache.clear()
	lastSkipReport = 0
}

/**
 * USD per one whole `symbol`, from the orderbook's best rate against a $1 stable. Null when the
 * orderbook quotes no such pair, is not configured, or cannot be reached.
 *
 * It answers null rather than a made-up rate because whatever it returns is written into cumulative
 * USD volume, which is never recomputed. Answers are cached per symbol: a block's orders share one
 * request, and a token priced repeatedly costs at most one request per {@link PRICE_TTL_MS}.
 */
export async function fetchOrderbookUsdPrice(
	symbol: string,
	options: {
		/** Overrides the endpoint resolved from {@link ORDERBOOK_URL_VAR}. */
		url?: string
		/** Wall clock, injectable for tests. */
		now?: number
	} = {},
): Promise<Decimal | null> {
	const now = options.now ?? Date.now()
	const url = options.url ?? orderbookEndpoint(GRAPHQL_PATH)
	if (!url) {
		if (now - lastSkipReport >= SKIP_REPORT_INTERVAL_MS) {
			lastSkipReport = now
			logger.info(`[orderbook-rates] Not pricing: ${ORDERBOOK_URL_VAR} is not set`)
		}
		return null
	}
	if (!symbol || symbol.length > MAX_SYMBOL_LENGTH) return null

	const cached = cache.get(symbol)
	if (cached && cached.expiresAt > now) return cached.price

	const entry: CacheEntry = { price: Promise.resolve(null), expiresAt: Number.POSITIVE_INFINITY }
	entry.price = fetchPrice(url, symbol).then((outcome) => {
		entry.expiresAt = now + (outcome.failed ? FAILURE_TTL_MS : PRICE_TTL_MS)
		return outcome.price
	})
	cache.set(symbol, entry)
	return entry.price
}

/**
 * One request. Never throws for the orderbook's sake: an unreachable or misbehaving orderbook is not
 * an indexing error, so it is logged and the token goes unpriced.
 */
async function fetchPrice(url: string, symbol: string): Promise<Outcome> {
	let body: unknown
	try {
		const response = await safeFetch(url, {
			method: "POST",
			headers: { "content-type": "application/json", accept: "application/json" },
			body: JSON.stringify({ query: QUERY, variables: { symbol } }),
			timeoutMs: FETCH_TIMEOUT_MS,
		})
		if (!response.ok) {
			logger.warn(`[orderbook-rates] ${url} answered ${response.status} pricing ${symbol}; retrying later`)
			return FAILED
		}
		body = await response.json()
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.warn(`[orderbook-rates] Pricing ${symbol} from ${url} failed; retrying later: ${message}`)
		return FAILED
	}

	const envelope = body as { data?: Record<string, unknown> | null; errors?: unknown } | null
	if (!envelope || typeof envelope !== "object" || (!("data" in envelope) && !("errors" in envelope))) {
		logger.warn(`[orderbook-rates] ${url} returned an unrecognised body pricing ${symbol}; retrying later`)
		return FAILED
	}

	const data = envelope.data
	for (const [index, stable] of STABLE_SYMBOLS.entries()) {
		const price = data ? usdPrice(data[`q${index}`], stable) : null
		if (price) return { price, failed: false }
	}
	// An unlisted token is the ordinary case here — the orderbook answers `no book trades X for
	// USDC` — so this says what it was told rather than warning. The caller reports the unpriced
	// token itself.
	if (Array.isArray(envelope.errors) && envelope.errors.length > 0) {
		const messages = envelope.errors
			.map((error) => (error as { message?: unknown })?.message)
			.filter((message): message is string => typeof message === "string")
			.join("; ")
		logger.info(`[orderbook-rates] ${url} priced no ${symbol}: ${messages}`)
	}
	return NO_PRICE
}

/**
 * One `bestRate` answer as USD per one whole token, given the `stable` that alias asked about.
 *
 * `rate` is quote per 1 base at 1e18, and the entry names both symbols, so where the stable sits
 * decides the arithmetic — not the side, which is the same book read either way:
 * - the stable is the quote, so the rate is dollars per 1 token, already the price;
 * - the stable is the base, so the rate is tokens per $1, and the price is its reciprocal. The
 *   USDC/cNGN book quoting 1,500.5 cNGN per USDC prices one cNGN at 1/1500.5 dollars.
 *
 * The symbols are compared against the stable this indexer asked for, not against the token's own
 * symbol, because the orderbook matches a pair on the exact string it was sent.
 */
function usdPrice(entry: unknown, stable: string): Decimal | null {
	const { base, quote, rate } = (entry ?? {}) as { base?: unknown; quote?: unknown; rate?: unknown }
	const stableIsQuote = quote === stable
	// Neither: not the pair that was asked about, so the rate is in units this cannot name.
	if (!stableIsQuote && base !== stable) return null
	if (typeof rate !== "string" && typeof rate !== "number") return null

	let value: Decimal
	try {
		value = new Decimal(rate)
	} catch {
		return null
	}
	// A zero or negative rate is not a price, and inverting it would not be one either.
	if (!value.isFinite() || value.lte(0)) return null

	return stableIsQuote ? value.div(WAD) : WAD.div(value)
}
