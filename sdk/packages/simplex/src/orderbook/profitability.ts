import { Decimal as BaseDecimal } from "decimal.js"
import { normalizeSymbol, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import type { LimitOrderFillRecord } from "@/data/types"
import { ORDERBOOK_SCALE } from "./amounts"

/**
 * A 1e18 amount of a nine-figure order carries 27 significant digits, more than the library's
 * default of 20. Cloned rather than configured, so no other user of it is affected.
 */
const Decimal = BaseDecimal.clone({ precision: 40 })
type Decimal = InstanceType<typeof Decimal>

/** The stretch of time a summary covers. Each is cut into buckets of one size. */
export type ProfitPeriod = "7d" | "30d" | "12w" | "12m" | "all"
export type ProfitBucket = "day" | "week" | "month" | "year"

/** How each period is bucketed, and how many buckets it holds. "all" runs from the first fill. */
export const PROFIT_PERIODS: Record<ProfitPeriod, { bucket: ProfitBucket; count: number | null }> = {
	"7d": { bucket: "day", count: 7 },
	"30d": { bucket: "day", count: 30 },
	"12w": { bucket: "week", count: 12 },
	"12m": { bucket: "month", count: 12 },
	all: { bucket: "year", count: null },
}

export function isProfitPeriod(value: string): value is ProfitPeriod {
	return value in PROFIT_PERIODS
}

/** What a stretch of fills came to, in dollars. Books with no dollar-stable leg are left out. */
export interface ProfitFigures {
	/** Profit realized by fills that closed volume bought or sold earlier. */
	realizedUsd: number
	/** The volume those fills closed. */
	matchedUsd: number
	boughtUsd: number
	soldUsd: number
	/** `realizedUsd` over `matchedUsd`, as a percentage. Null when nothing was matched. */
	spreadPct: number | null
	buys: number
	sells: number
}

export interface ProfitBucketFigures extends ProfitFigures {
	/** When the bucket starts, in milliseconds since the epoch. */
	start: number
}

/** One book's buys against its sells over the period, in the book's own tokens. */
export interface BookProfit {
	book: string
	base: string
	quote: string
	/** Base bought and sold in the period. */
	bought: number
	sold: number
	/** Quote per base, averaged over the period's buys and its sells. Null for a side with none. */
	averageBuy: number | null
	averageSell: number | null
	/** Base closed in the period: bought volume sold again, or sold volume bought back. */
	matched: number
	/** Profit realized in the period, in the quote token. */
	realized: number
	/** Null when neither token is a dollar stable. */
	realizedUsd: number | null
	spreadPct: number | null
	/** Base held open now: positive is bought and not yet sold, negative is sold and not yet bought back. */
	position: number
	positionUsd: number | null
}

export interface Profitability {
	period: ProfitPeriod
	bucket: ProfitBucket
	/** The start of the first bucket and the moment the summary was taken, in milliseconds. */
	from: number
	to: number
	totals: ProfitFigures
	/** One entry per bucket, oldest first, including buckets nothing happened in. */
	series: ProfitBucketFigures[]
	books: BookProfit[]
	/** The dollar value of every book's open position, whichever way it faces. */
	openPositionUsd: number
	/** Fills in the period priced at their order's rate, because they kept no record of what they took in. */
	estimatedFills: number
	/** Books in the period that no dollar figure includes. */
	unpricedBooks: string[]
}

const MINUTE = 60_000
const DAY = 86_400_000

/**
 * Where the bucket holding `local` starts. `local` and the answer are both on the viewer's clock,
 * carried as UTC so the date arithmetic needs no timezone database.
 */
function bucketStart(local: number, bucket: ProfitBucket): number {
	const date = new Date(local)
	const year = date.getUTCFullYear()
	const month = date.getUTCMonth()
	if (bucket === "year") return Date.UTC(year, 0, 1)
	if (bucket === "month") return Date.UTC(year, month, 1)
	const day = Date.UTC(year, month, date.getUTCDate())
	// Weeks start on Monday. `getUTCDay` counts from Sunday.
	return bucket === "day" ? day : day - ((date.getUTCDay() + 6) % 7) * DAY
}

/** The bucket `steps` after the one starting at `start`. Negative steps go back. */
function shiftBucket(start: number, bucket: ProfitBucket, steps: number): number {
	if (bucket === "day") return start + steps * DAY
	if (bucket === "week") return start + steps * 7 * DAY
	const date = new Date(start)
	return bucket === "month"
		? Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + steps, 1)
		: Date.UTC(date.getUTCFullYear() + steps, 0, 1)
}

/** Milliseconds since the epoch for the store's "YYYY-MM-DD HH:MM:SS" UTC timestamps. */
function parseUtc(value: string): number {
	return Date.parse(`${value.replace(" ", "T")}Z`)
}

function scaled(value: string): Decimal {
	return new Decimal(value).div(ORDERBOOK_SCALE.toString())
}

/** What a fill moved, in whole tokens of its book: base one way, quote the other. */
interface Trade {
	base: Decimal
	quote: Decimal
	/** No record of what the fill took in: the other side is worked out from the order's rate. */
	estimated: boolean
}

/**
 * A fill as a trade on its book. A buy pays quote out and takes base in, a sell the reverse. The
 * side taken in is the escrow the fill released, which at the swapper's rate is never worse than
 * the order's own; a fill that kept none is priced at the order's rate, the least it could have
 * taken. Null when the fill cannot be priced at all.
 */
function tradeOf(fill: LimitOrderFillRecord): Trade | null {
	try {
		const paid = scaled(fill.amount)
		const taken = fill.amountIn === null ? null : scaled(fill.amountIn)
		const price = scaled(fill.price)
		if (!paid.gt(0) || !price.gt(0) || (taken !== null && !taken.gt(0))) return null
		const buying = fill.side === "BID"
		return buying
			? { quote: paid, base: taken ?? paid.div(price), estimated: taken === null }
			: { base: paid, quote: taken ?? paid.mul(price), estimated: taken === null }
	} catch {
		return null
	}
}

/** What one fill realized: nothing unless it closed volume the book already held. */
interface Realization {
	/** Base closed. */
	matched: Decimal
	/** Profit in the quote token. */
	realized: Decimal
}

/**
 * One book's open position, kept at average cost.
 *
 * A buy that adds to bought volume, or a sell that adds to sold volume, moves the average and
 * realizes nothing. A fill the other way closes what is held, oldest and newest alike, at that
 * average: the difference between its rate and the average is the profit. Volume past what was
 * held opens a position the other way at the fill's own rate.
 */
class Position {
	/** Base held: positive is bought and unsold, negative is sold and not bought back. */
	base = new Decimal(0)
	/** Quote per base the held volume was opened at, on average. */
	cost = new Decimal(0)

	apply(trade: Trade, buying: boolean): Realization {
		const rate = trade.quote.div(trade.base)
		const signed = buying ? trade.base : trade.base.neg()
		const none = { matched: new Decimal(0), realized: new Decimal(0) }

		// Same direction as what is held, or nothing held: the position grows and its average moves.
		if (this.base.isZero() || this.base.isPositive() === signed.isPositive()) {
			const held = this.base.abs()
			this.cost = held.mul(this.cost).add(trade.quote).div(held.add(trade.base))
			this.base = this.base.add(signed)
			return none
		}

		const matched = Decimal.min(trade.base, this.base.abs())
		// Selling bought volume earns the fill's rate over the average; buying back sold volume
		// earns the average over the fill's rate.
		const realized = matched.mul(buying ? this.cost.sub(rate) : rate.sub(this.cost))
		const opened = trade.base.sub(matched)
		this.base = this.base.add(signed)
		if (opened.gt(0)) this.cost = rate
		else if (this.base.isZero()) this.cost = new Decimal(0)
		return { matched, realized }
	}
}

/**
 * A book whose two tokens are one asset holds no position: each fill takes in a little more than
 * it pays out, and that difference is its profit there and then.
 */
function inKind(trade: Trade, buying: boolean): Realization {
	const [paid, taken] = buying ? [trade.quote, trade.base] : [trade.base, trade.quote]
	return { matched: trade.base, realized: taken.sub(paid) }
}

/** Running sums for one stretch of time. Dollar sums only count books that can be priced. */
class Sums {
	realizedUsd = new Decimal(0)
	matchedUsd = new Decimal(0)
	boughtUsd = new Decimal(0)
	soldUsd = new Decimal(0)
	buys = 0
	sells = 0

	figures(): ProfitFigures {
		return {
			realizedUsd: this.realizedUsd.toNumber(),
			matchedUsd: this.matchedUsd.toNumber(),
			boughtUsd: this.boughtUsd.toNumber(),
			soldUsd: this.soldUsd.toNumber(),
			spreadPct: percent(this.realizedUsd, this.matchedUsd),
			buys: this.buys,
			sells: this.sells,
		}
	}
}

function percent(part: Decimal, whole: Decimal): number | null {
	return whole.gt(0) ? part.div(whole).mul(100).toNumber() : null
}

/** A book across the whole history (its position) and across the period (everything else). */
class BookLedger {
	position = new Position()
	bought = new Decimal(0)
	boughtQuote = new Decimal(0)
	sold = new Decimal(0)
	soldQuote = new Decimal(0)
	matched = new Decimal(0)
	matchedQuote = new Decimal(0)
	realized = new Decimal(0)
	realizedUsd = new Decimal(0)
	matchedUsd = new Decimal(0)
	inPeriod = false
	readonly sameAsset: boolean
	private readonly stable: "base" | "quote" | null

	constructor(
		readonly id: string,
		readonly base: string,
		readonly quote: string,
	) {
		this.sameAsset = normalizeSymbol(base) === normalizeSymbol(quote)
		this.stable = USD_STABLE_SYMBOLS.has(normalizeSymbol(quote))
			? "quote"
			: USD_STABLE_SYMBOLS.has(normalizeSymbol(base))
				? "base"
				: null
	}

	/**
	 * Dollars per unit of quote, given the rate of the fill being valued. A book is priced through
	 * whichever of its tokens is a dollar stable: the quote directly, or the base through the rate.
	 */
	usdPerQuote(rate: Decimal): Decimal | null {
		if (this.stable === "quote") return new Decimal(1)
		return this.stable === "base" ? new Decimal(1).div(rate) : null
	}

	/**
	 * What the open position is worth in dollars, signed like the position. Held base that is
	 * itself dollars counts at face; otherwise it counts at what it cost, not at what it would
	 * fetch now.
	 */
	positionUsd(): Decimal | null {
		const held = this.position.base
		if (this.stable === "base") return held
		return this.stable === "quote" ? held.mul(this.position.cost) : null
	}
}

/**
 * Buys against sells, as profit.
 *
 * Every fill of every limit order is replayed in order, per book, against a position kept at
 * average cost. A fill realizes profit when it closes volume the book already held, and that
 * profit is counted in the bucket the closing fill falls in. Fills from before the period still
 * run, since they are what set the cost the period's fills close against.
 *
 * Profit is worked out in the book's quote token and shown in dollars at the rate of the fill
 * that realized it. Open volume is not marked to market, and network fees are not counted.
 *
 * `tzOffsetMinutes` is what JavaScript's `getTimezoneOffset` returns on the viewer's clock, so
 * a day starts at their midnight rather than at UTC's.
 */
export function summarizeProfit(
	fills: readonly LimitOrderFillRecord[],
	options: { period: ProfitPeriod; now: number; tzOffsetMinutes?: number },
): Profitability {
	const { period, now } = options
	const offset = (options.tzOffsetMinutes ?? 0) * MINUTE
	const { bucket, count } = PROFIT_PERIODS[period]

	const ordered = fills
		.map((fill) => ({ fill, at: parseUtc(fill.filledAt) }))
		.filter((entry) => Number.isFinite(entry.at) && entry.at <= now)
		.sort((a, b) => a.at - b.at || a.fill.id - b.fill.id)

	// Bucket starts on the viewer's clock, oldest first. "all" opens at the first fill's bucket.
	const last = bucketStart(now - offset, bucket)
	const first =
		count === null ? bucketStart((ordered[0]?.at ?? now) - offset, bucket) : shiftBucket(last, bucket, 1 - count)
	const starts: number[] = []
	for (let start = first; start <= last; start = shiftBucket(start, bucket, 1)) starts.push(start)

	const totals = new Sums()
	const buckets = new Map(starts.map((start) => [start, new Sums()]))
	const books = new Map<string, BookLedger>()
	let estimatedFills = 0

	for (const { fill, at } of ordered) {
		const trade = tradeOf(fill)
		if (!trade) continue
		let book = books.get(fill.book)
		if (!book) {
			book = new BookLedger(fill.book, fill.base, fill.quote)
			books.set(fill.book, book)
		}

		const buying = fill.side === "BID"
		const rate = trade.quote.div(trade.base)
		const { matched, realized } = book.sameAsset ? inKind(trade, buying) : book.position.apply(trade, buying)

		const sums = buckets.get(bucketStart(at - offset, bucket))
		if (!sums) continue
		book.inPeriod = true
		if (trade.estimated) estimatedFills += 1

		if (buying) {
			book.bought = book.bought.add(trade.base)
			book.boughtQuote = book.boughtQuote.add(trade.quote)
		} else {
			book.sold = book.sold.add(trade.base)
			book.soldQuote = book.soldQuote.add(trade.quote)
		}
		book.matched = book.matched.add(matched)
		book.matchedQuote = book.matchedQuote.add(matched.mul(rate))
		book.realized = book.realized.add(realized)

		const usd = book.usdPerQuote(rate)
		for (const sum of [totals, sums]) {
			if (buying) sum.buys += 1
			else sum.sells += 1
			if (!usd) continue
			const volume = trade.quote.mul(usd)
			if (buying) sum.boughtUsd = sum.boughtUsd.add(volume)
			else sum.soldUsd = sum.soldUsd.add(volume)
			sum.matchedUsd = sum.matchedUsd.add(matched.mul(rate).mul(usd))
			sum.realizedUsd = sum.realizedUsd.add(realized.mul(usd))
		}
		if (usd) {
			book.matchedUsd = book.matchedUsd.add(matched.mul(rate).mul(usd))
			book.realizedUsd = book.realizedUsd.add(realized.mul(usd))
		}
	}

	let openPositionUsd = new Decimal(0)
	const rows: BookProfit[] = []
	const unpricedBooks: string[] = []
	for (const book of books.values()) {
		const held = book.position.base
		const positionUsd = book.positionUsd()
		const priced = positionUsd !== null
		if (positionUsd) openPositionUsd = openPositionUsd.add(positionUsd.abs())
		if (!book.inPeriod && held.isZero()) continue
		if (!priced && book.inPeriod) unpricedBooks.push(book.id)

		rows.push({
			book: book.id,
			base: book.base,
			quote: book.quote,
			bought: book.bought.toNumber(),
			sold: book.sold.toNumber(),
			averageBuy: book.bought.gt(0) ? book.boughtQuote.div(book.bought).toNumber() : null,
			averageSell: book.sold.gt(0) ? book.soldQuote.div(book.sold).toNumber() : null,
			matched: book.matched.toNumber(),
			realized: book.realized.toNumber(),
			realizedUsd: priced ? book.realizedUsd.toNumber() : null,
			spreadPct: priced ? percent(book.realizedUsd, book.matchedUsd) : percent(book.realized, book.matchedQuote),
			position: held.toNumber(),
			positionUsd: positionUsd ? positionUsd.toNumber() : null,
		})
	}
	// The books that earned the most first, which is the order the page reads in.
	rows.sort((a, b) => (b.realizedUsd ?? 0) - (a.realizedUsd ?? 0) || a.book.localeCompare(b.book))

	return {
		period,
		bucket,
		from: first + offset,
		to: now,
		totals: totals.figures(),
		series: starts.map((start) => ({ start: start + offset, ...buckets.get(start)!.figures() })),
		books: rows,
		openPositionUsd: openPositionUsd.toNumber(),
		estimatedFills,
		unpricedBooks,
	}
}
