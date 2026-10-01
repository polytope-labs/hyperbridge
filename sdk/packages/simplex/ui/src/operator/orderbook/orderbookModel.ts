import type { OrderbookLevel, OrderbookSnapshot } from "../../types"

const SCALE = 10n ** 18n
const MIN_PRICE_DECIMALS = 2
const MAX_PRICE_DECIMALS = 6
const BPS_PRECISION = 1_000_000n

export interface DepthLevel extends OrderbookLevel {
	id: string
	side: "BID" | "ASK"
	cumulativeBase: string
	depthRatio: number
}

/** Sort and accumulate with integers: prices and token amounts exceed Number precision. */
export function depthLevels(levels: OrderbookLevel[], side: "BID" | "ASK"): DepthLevel[] {
	const sorted = [...levels].sort((a, b) => {
		const left = BigInt(a.price)
		const right = BigInt(b.price)
		const direction = left === right ? 0 : left > right ? -1 : 1
		return side === "BID" ? direction : -direction
	})
	const total = sorted.reduce((sum, level) => sum + BigInt(level.baseSize), 0n)
	let cumulative = 0n
	return sorted.map((level) => {
		cumulative += BigInt(level.baseSize)
		return {
			...level,
			id: `${side}-${level.fillChain}-${level.priceBucket}`,
			side,
			cumulativeBase: cumulative.toString(),
			depthRatio: total > 0n ? Number((cumulative * 10_000n) / total) / 100 : 0,
		}
	})
}

/** Top-of-book follows filtered levels; the book-wide best prices ignore route filters. */
export function topOfBook(snapshot: OrderbookSnapshot) {
	const bids = depthLevels(snapshot.bids, "BID")
	const asks = depthLevels(snapshot.asks, "ASK")
	const bid = bids[0]?.price
	const ask = asks[0]?.price
	const both = bid !== undefined && ask !== undefined
	const crossed = both && BigInt(bid) >= BigInt(ask)
	const mid = both ? (BigInt(bid) + BigInt(ask)) / 2n : null
	const spread = both && !crossed ? BigInt(ask) - BigInt(bid) : null
	const count = (levels: DepthLevel[]) => levels.reduce((total, level) => total + level.orderCount, 0)
	return {
		bids,
		asks,
		crossed,
		bestBid: bid ?? null,
		bestAsk: ask ?? null,
		mid: mid === null ? null : mid.toString(),
		spread: spread === null ? null : spread.toString(),
		spreadBps:
			spread === null || mid === null || mid <= 0n
				? null
				: Number((spread * BPS_PRECISION * 10_000n) / mid) / Number(BPS_PRECISION),
		bidLiquidity: snapshot.bidLiquidity,
		askLiquidity: snapshot.askLiquidity,
		bidOrderCount: count(bids),
		askOrderCount: count(asks),
		priceDecimals: priceDecimals(snapshot.granularity),
	}
}

export type TopOfBook = ReturnType<typeof topOfBook>

/**
 * Fraction digits that tell one price bucket from the next. Two is the floor: a whole-number
 * bucket at zero digits reads as a rounded figure rather than a price.
 */
export function priceDecimals(granularity: string | null | undefined): number {
	let raw = 0n
	try {
		raw = BigInt(granularity ?? "0")
	} catch {}
	if (raw <= 0n) return MIN_PRICE_DECIMALS
	const decimals = 19 - raw.toString().length
	return Math.min(MAX_PRICE_DECIMALS, Math.max(MIN_PRICE_DECIMALS, decimals))
}

/** A 1e18 fixed-point string as a float, for compact notation only; exact display uses formatFixed. */
function toNumber(raw: string | null | undefined): number | null {
	if (!raw) return null
	try {
		const value = BigInt(raw)
		const whole = Number(value / SCALE)
		const fraction = Number(value % SCALE) / 1e18
		return whole + fraction
	} catch {
		return null
	}
}

const group = (whole: bigint) => whole.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",")

/** Exactly `decimals` fraction digits, rounded to nearest, grouped: `1,361.50`. */
export function formatFixed(raw: string | null | undefined, decimals: number): string {
	if (!raw) return "—"
	let value: bigint
	try {
		value = BigInt(raw)
	} catch {
		return raw
	}
	const negative = value < 0n
	if (negative) value = -value
	const unit = 10n ** BigInt(18 - decimals)
	const rounded = (value + unit / 2n) / unit
	const divisor = 10n ** BigInt(decimals)
	const whole = group(rounded / divisor)
	const fraction = decimals > 0 ? `.${(rounded % divisor).toString().padStart(decimals, "0")}` : ""
	return `${negative ? "-" : ""}${whole}${fraction}`
}

export const formatPrice = formatFixed

const COMPACT = new Intl.NumberFormat("en-US", {
	notation: "compact",
	maximumFractionDigits: 1,
	roundingMode: "floor",
} as Intl.NumberFormatOptions)

/** Sizes keep two decimals so the columns line up; millions go compact. */
export function formatSize(raw: string | null | undefined): string {
	if (!raw) return "—"
	let value: bigint
	try {
		value = BigInt(raw)
	} catch {
		return raw
	}
	const abs = value < 0n ? -value : value
	if (abs >= 1_000_000n * SCALE) return COMPACT.format(toNumber(raw) ?? 0)
	return formatFixed(raw, 2)
}

const BPS = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 })

export function formatSpreadBps(bps: number | null): string {
	return bps === null ? "—" : `${BPS.format(bps)} bps`
}

export function formatRelativeTime(timestamp: number, now: number): string {
	let remaining = Math.max(0, Math.round((now - timestamp) / 1000))
	const days = Math.floor(remaining / 86_400)
	remaining %= 86_400
	const hours = Math.floor(remaining / 3_600)
	remaining %= 3_600
	const minutes = Math.floor(remaining / 60)
	const seconds = remaining % 60
	const parts = [
		days ? `${days}d` : "",
		hours ? `${hours}hr` : "",
		minutes ? `${minutes}m` : "",
		seconds || (!days && !hours && !minutes) ? `${seconds}s` : "",
	]
	return `${parts.filter(Boolean).join("")} ago`
}
