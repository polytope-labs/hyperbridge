import { Decimal as BaseDecimal } from "decimal.js"
import { normalizeSymbol } from "@/config/asset-registry"
import { parseUtc } from "@/data/inventory"
import type { LimitOrderFillRecord } from "@/data/types"
import { ORDERBOOK_SCALE } from "./amounts"
import { type UsdEdge, usdFactorsFrom } from "./usd"

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

const MINUTE = 60_000
const DAY = 86_400_000

/**
 * The furthest a record of inventory may be from the instant it is asked to describe. Past that,
 * too much may have moved in and out that no fill or send accounts for, and the figure is left
 * unknown rather than guessed.
 */
export const INVENTORY_REACH_MS = 7 * DAY

/** What the solver held at one moment, as whole tokens per symbol. */
export interface InventoryRecord {
	/** When the balances were read, in milliseconds since the epoch. */
	at: number
	balances: Record<string, number>
	/** True for the balances as they stand now, rather than a stored snapshot. */
	live?: boolean
}

/** Tokens that left the solver other than through a fill: an operator's send. */
export interface InventoryOutflow {
	at: number
	symbol: string
	amount: number
}

/** What a stretch of fills came to, in dollars at the latest rate. Unpriced tokens are left out. */
export interface ProfitFigures {
	/** What the stretch's fills added to inventory, less what they took out of it. */
	profitUsd: number
	/** The inventory held when the stretch began. Null when nothing within reach of then records it. */
	startInventoryUsd: number | null
	/** `profitUsd` over `startInventoryUsd`, as a percentage. Null without a starting inventory. */
	returnPct: number | null
	boughtUsd: number
	soldUsd: number
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
	/** How far the average sell sits above the average buy, as a percentage. Null without both. */
	spreadPct: number | null
	/** What the period's fills did to the base and the quote held: positive is more than before. */
	baseChange: number
	quoteChange: number
	/** The rate the book's tokens are valued at: midway between its latest buy and latest sell. */
	rate: number | null
	/** Both changes in dollars. Null when the book's tokens have no dollar price. */
	profitUsd: number | null
}

/** The inventory a period began with, token by token. */
export interface StartingInventory {
	/** The instant it describes, in milliseconds since the epoch. */
	at: number
	/** A stored snapshot, or the balances as they stand now. Either is adjusted by what happened between. */
	source: "snapshot" | "balances"
	/** When that record was taken. */
	recordedAt: number
	usd: number
	tokens: Array<{ symbol: string; amount: number; usd: number | null }>
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
	/** Null when no record of inventory lies within {@link INVENTORY_REACH_MS} of the period's start. */
	startInventory: StartingInventory | null
	/** Fills in the period priced at their order's rate, because they kept no record of what they took in. */
	estimatedFills: number
	/** Tokens the period touched that have no dollar price, and so are in none of the dollar figures. */
	unpricedTokens: string[]
}

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

function scaled(value: string): Decimal {
	return new Decimal(value).div(ORDERBOOK_SCALE.toString())
}

/** A fill as what it moved, in whole tokens: base one way and quote the other. */
interface Trade {
	at: number
	book: string
	/** Normalised symbols, which is what holdings are keyed by. */
	base: string
	quote: string
	buying: boolean
	baseAmount: Decimal
	quoteAmount: Decimal
	/** No record of what the fill took in: the other side is worked out from the order's rate. */
	estimated: boolean
}

/**
 * A fill as a trade on its book. A buy pays quote out and takes base in, a sell the reverse. The
 * side taken in is the escrow the fill released, which at the swapper's rate is never worse than
 * the order's own; a fill that kept none is priced at the order's rate, the least it could have
 * taken. Null when the fill cannot be priced at all.
 */
function tradeOf(fill: LimitOrderFillRecord, at: number): Trade | null {
	try {
		const paid = scaled(fill.amount)
		const taken = fill.amountIn === null ? null : scaled(fill.amountIn)
		const price = scaled(fill.price)
		if (!paid.gt(0) || !price.gt(0) || (taken !== null && !taken.gt(0))) return null
		const buying = fill.side === "BID"
		const [baseAmount, quoteAmount] = buying ? [taken ?? paid.div(price), paid] : [paid, taken ?? paid.mul(price)]
		return {
			at,
			book: fill.book,
			base: normalizeSymbol(fill.base),
			quote: normalizeSymbol(fill.quote),
			buying,
			baseAmount,
			quoteAmount,
			estimated: taken === null,
		}
	} catch {
		return null
	}
}

/** Tokens per symbol: a holding, or a change in one. */
type Holdings = Map<string, Decimal>

function add(holdings: Holdings, symbol: string, amount: Decimal): void {
	holdings.set(symbol, (holdings.get(symbol) ?? new Decimal(0)).add(amount))
}

/** What a trade does to what is held: the side taken in goes up, the side paid out goes down. */
function apply(holdings: Holdings, trade: Trade, sign: 1 | -1): void {
	const base = trade.buying ? trade.baseAmount : trade.baseAmount.neg()
	const quote = trade.buying ? trade.quoteAmount.neg() : trade.quoteAmount
	add(holdings, trade.base, base.mul(sign))
	add(holdings, trade.quote, quote.mul(sign))
}

/** Running sums for one stretch of time. */
class Sums {
	change: Holdings = new Map()
	boughtUsd = new Decimal(0)
	soldUsd = new Decimal(0)
	buys = 0
	sells = 0
}

/** A book over the period: what it bought and sold, and at what rates. */
class BookLedger {
	bought = new Decimal(0)
	boughtQuote = new Decimal(0)
	sold = new Decimal(0)
	soldQuote = new Decimal(0)
	/** The rates of the book's latest buy and latest sell, over all of its history. */
	latestBuy: Decimal | null = null
	latestSell: Decimal | null = null
	inPeriod = false

	constructor(
		readonly id: string,
		readonly base: string,
		readonly quote: string,
	) {}

	/**
	 * The rate the book is valued at. Midway between the latest buy and the latest sell, because
	 * either alone sits half a spread to one side and would price open volume against whichever
	 * way the book happened to trade last.
	 */
	rate(): Decimal | null {
		if (this.latestBuy && this.latestSell) return this.latestBuy.add(this.latestSell).div(2)
		return this.latestBuy ?? this.latestSell
	}
}

/**
 * What the solver held at `instant`, from the nearest record within reach of it.
 *
 * A record is rarely taken at the instant asked about, so it is carried there by what is known to
 * have happened between: fills, which moved tokens both ways, and sends, which took them out. A
 * record from after the instant has those undone; one from before has them applied. A deposit
 * from outside is on no record, so it reads as held from the start of whatever gap it fell in,
 * which is why the gap is capped. Null when no record lies within reach.
 */
function inventoryAt(
	instant: number,
	records: readonly InventoryRecord[],
	trades: readonly Trade[],
	outflows: readonly InventoryOutflow[],
): { held: Holdings; record: InventoryRecord } | null {
	let record: InventoryRecord | undefined
	let gap = Number.POSITIVE_INFINITY
	for (const candidate of records) {
		const distance = Math.abs(candidate.at - instant)
		if (distance > INVENTORY_REACH_MS) continue
		// A stored snapshot is preferred to the live balances at the same distance: it is a fact
		// about then, where the live balances are a fact about now.
		if (distance < gap || (distance === gap && record?.live && !candidate.live)) {
			record = candidate
			gap = distance
		}
	}
	if (!record) return null

	const held: Holdings = new Map()
	for (const [symbol, amount] of Object.entries(record.balances)) {
		if (Number.isFinite(amount)) add(held, normalizeSymbol(symbol), new Decimal(amount))
	}

	const after = record.at >= instant
	const [from, to] = after ? [instant, record.at] : [record.at, instant]
	// Undoing what followed the instant, or applying what led up to it.
	const sign = after ? -1 : 1
	for (const trade of trades) {
		if (trade.at >= from && trade.at < to) apply(held, trade, sign)
	}
	for (const outflow of outflows) {
		if (outflow.at >= from && outflow.at < to) {
			add(held, normalizeSymbol(outflow.symbol), new Decimal(outflow.amount).mul(-sign))
		}
	}
	// A deposit after the instant can leave a token looking overdrawn before it; nothing is ever
	// held in the negative.
	for (const [symbol, amount] of held) if (amount.isNegative()) held.set(symbol, new Decimal(0))
	return { held, record }
}

/**
 * Profit from buys and sells, as the change they made to what the solver holds.
 *
 * Every fill in the period took one token in and paid another out. Added up per token, that is
 * what the period did to inventory, and its value in dollars is the period's profit: positive
 * when the fills left the solver holding more than they found it with. Each token is valued at
 * one rate throughout, the latest, so a figure compares like with like and a move in the rate
 * shows only on volume the period left open. Buying low and selling high in equal measure leaves
 * the base where it was and the quote up by the spread; buying alone leaves base up and quote
 * down, worth the difference between what was paid and the latest rate.
 *
 * The profit is compared with the inventory the period began with, taken from the nearest
 * snapshot or, failing that, from today's balances, and in either case only from within
 * {@link INVENTORY_REACH_MS} of the period's start. Network fees are not counted.
 *
 * `tzOffsetMinutes` is what JavaScript's `getTimezoneOffset` returns on the viewer's clock, so
 * a day starts at their midnight rather than at UTC's.
 */
export function summarizeProfit(
	fills: readonly LimitOrderFillRecord[],
	options: {
		period: ProfitPeriod
		now: number
		tzOffsetMinutes?: number
		/** Stored snapshots and, when they could be read, the balances as they stand now. */
		inventory?: readonly InventoryRecord[]
		outflows?: readonly InventoryOutflow[]
	},
): Profitability {
	const { period, now } = options
	const offset = (options.tzOffsetMinutes ?? 0) * MINUTE
	const { bucket, count } = PROFIT_PERIODS[period]
	const records = (options.inventory ?? []).filter((record) => record.at <= now)
	const outflows = options.outflows ?? []

	const trades: Trade[] = []
	for (const fill of fills) {
		const at = parseUtc(fill.filledAt)
		if (!Number.isFinite(at) || at > now) continue
		const trade = tradeOf(fill, at)
		if (trade) trades.push(trade)
	}
	trades.sort((a, b) => a.at - b.at)

	// Bucket starts on the viewer's clock, oldest first. "all" opens at the first fill's bucket.
	const last = bucketStart(now - offset, bucket)
	const first =
		count === null ? bucketStart((trades[0]?.at ?? now) - offset, bucket) : shiftBucket(last, bucket, 1 - count)
	const starts: number[] = []
	for (let start = first; start <= last; start = shiftBucket(start, bucket, 1)) starts.push(start)

	// Every book's latest rates come from its whole history: a book that did not trade in the
	// period still prices the tokens the period's fills and the starting inventory are made of.
	const books = new Map<string, BookLedger>()
	// How each symbol is spelled, for showing: the balances' spelling first, then a book's.
	const spelling = new Map<string, string>()
	for (const record of records) {
		for (const symbol of Object.keys(record.balances)) {
			if (!spelling.has(normalizeSymbol(symbol))) spelling.set(normalizeSymbol(symbol), symbol)
		}
	}
	for (const fill of fills) {
		for (const symbol of [fill.base, fill.quote]) {
			if (!spelling.has(normalizeSymbol(symbol))) spelling.set(normalizeSymbol(symbol), symbol)
		}
	}
	for (const trade of trades) {
		let book = books.get(trade.book)
		if (!book) {
			book = new BookLedger(trade.book, trade.base, trade.quote)
			books.set(trade.book, book)
		}
		const rate = trade.quoteAmount.div(trade.baseAmount)
		if (trade.buying) book.latestBuy = rate
		else book.latestSell = rate
	}

	// One dollar price per token, walked out from the dollar stables through each book's rate.
	const edges: UsdEdge[] = []
	for (const book of books.values()) {
		const rate = book.rate()
		// A book whose two tokens are one asset says nothing about what that asset is worth.
		if (rate && book.base !== book.quote) {
			edges.push({ base: book.base, quote: book.quote, rate: new BaseDecimal(rate.toString()) })
		}
	}
	const factors = usdFactorsFrom(edges)
	const priceOf = (symbol: string): Decimal | null => {
		const factor = factors.get(symbol)
		return factor ? new Decimal(factor.toString()) : null
	}
	const unpriced = new Set<string>()
	/** The dollar value of a set of holdings or changes, noting any token it had to leave out. */
	const valueOf = (holdings: Holdings): Decimal => {
		let total = new Decimal(0)
		for (const [symbol, amount] of holdings) {
			if (amount.isZero()) continue
			const price = priceOf(symbol)
			if (price) total = total.add(amount.mul(price))
			else unpriced.add(symbol)
		}
		return total
	}

	const totals = new Sums()
	const buckets = new Map(starts.map((start) => [start, new Sums()]))
	let estimatedFills = 0

	for (const trade of trades) {
		const sums = buckets.get(bucketStart(trade.at - offset, bucket))
		if (!sums) continue
		const book = books.get(trade.book)
		if (!book) continue
		book.inPeriod = true
		if (trade.estimated) estimatedFills += 1

		if (trade.buying) {
			book.bought = book.bought.add(trade.baseAmount)
			book.boughtQuote = book.boughtQuote.add(trade.quoteAmount)
		} else {
			book.sold = book.sold.add(trade.baseAmount)
			book.soldQuote = book.soldQuote.add(trade.quoteAmount)
		}

		// The size of the trade in dollars, from whichever of its tokens has a price.
		const basePrice = priceOf(trade.base)
		const quotePrice = priceOf(trade.quote)
		const volume = basePrice
			? trade.baseAmount.mul(basePrice)
			: quotePrice
				? trade.quoteAmount.mul(quotePrice)
				: new Decimal(0)
		for (const sum of [totals, sums]) {
			apply(sum.change, trade, 1)
			if (trade.buying) {
				sum.buys += 1
				sum.boughtUsd = sum.boughtUsd.add(volume)
			} else {
				sum.sells += 1
				sum.soldUsd = sum.soldUsd.add(volume)
			}
		}
	}

	/** A stretch's sums against the inventory it began with. */
	const figures = (sums: Sums, startsAt: number): ProfitFigures => {
		const profit = valueOf(sums.change)
		const began = inventoryAt(startsAt, records, trades, outflows)
		const inventory = began ? valueOf(began.held) : null
		return {
			profitUsd: profit.toNumber(),
			startInventoryUsd: inventory ? inventory.toNumber() : null,
			returnPct: inventory?.gt(0) ? profit.div(inventory).mul(100).toNumber() : null,
			boughtUsd: sums.boughtUsd.toNumber(),
			soldUsd: sums.soldUsd.toNumber(),
			buys: sums.buys,
			sells: sums.sells,
		}
	}

	const from = first + offset
	const began = inventoryAt(from, records, trades, outflows)
	const startInventory: StartingInventory | null = began && {
		at: from,
		source: began.record.live ? "balances" : "snapshot",
		recordedAt: began.record.at,
		usd: valueOf(began.held).toNumber(),
		tokens: [...began.held]
			.filter(([, amount]) => amount.gt(0))
			.map(([symbol, amount]) => {
				const price = priceOf(symbol)
				return {
					symbol: spelling.get(symbol) ?? symbol,
					amount: amount.toNumber(),
					usd: price ? amount.mul(price).toNumber() : null,
				}
			})
			.sort((a, b) => (b.usd ?? 0) - (a.usd ?? 0) || a.symbol.localeCompare(b.symbol)),
	}

	const rows: BookProfit[] = []
	for (const book of books.values()) {
		if (!book.inPeriod) continue
		const averageBuy = book.bought.gt(0) ? book.boughtQuote.div(book.bought) : null
		const averageSell = book.sold.gt(0) ? book.soldQuote.div(book.sold) : null
		const baseChange = book.bought.sub(book.sold)
		const quoteChange = book.soldQuote.sub(book.boughtQuote)
		const basePrice = priceOf(book.base)
		const quotePrice = priceOf(book.quote)
		rows.push({
			book: book.id,
			base: spelling.get(book.base) ?? book.base,
			quote: spelling.get(book.quote) ?? book.quote,
			bought: book.bought.toNumber(),
			sold: book.sold.toNumber(),
			averageBuy: averageBuy ? averageBuy.toNumber() : null,
			averageSell: averageSell ? averageSell.toNumber() : null,
			spreadPct: averageBuy && averageSell ? averageSell.div(averageBuy).sub(1).mul(100).toNumber() : null,
			baseChange: baseChange.toNumber(),
			quoteChange: quoteChange.toNumber(),
			rate: book.rate()?.toNumber() ?? null,
			profitUsd:
				basePrice && quotePrice ? baseChange.mul(basePrice).add(quoteChange.mul(quotePrice)).toNumber() : null,
		})
	}
	// The books that earned the most first, which is the order the page reads in.
	rows.sort((a, b) => (b.profitUsd ?? 0) - (a.profitUsd ?? 0) || a.book.localeCompare(b.book))

	return {
		period,
		bucket,
		from,
		to: now,
		totals: figures(totals, from),
		series: starts.map((start) => {
			const sums = buckets.get(start) ?? new Sums()
			return { start: start + offset, ...figures(sums, start + offset) }
		}),
		books: rows,
		startInventory,
		estimatedFills,
		unpricedTokens: [...unpriced].map((symbol) => spelling.get(symbol) ?? symbol).sort(),
	}
}
