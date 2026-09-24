import type { CreateLimitOrderRequest, LimitOrder } from "../../types"

/** The orderbook keeps every amount and price at 1e18, whatever the token's own decimals. */
const SCALE = 10n ** 18n

/**
 * A 1e18 figure as whole tokens, trimmed of trailing zeros.
 *
 * The operator states amounts in whole tokens and reads them back the same way:
 * what the orderbook normalises to is not their problem. Digits past `maxFraction`
 * are cut by default, so an amount left is never overstated.
 */
export function fromScaled(value: string, maxFraction = 6, rounding: "down" | "nearest" = "down"): string {
	let raw: bigint
	try {
		raw = BigInt(value)
	} catch {
		return value
	}
	// Half of the last digit shown, so the cut below lands on the nearest.
	if (rounding === "nearest") raw += 10n ** BigInt(18 - maxFraction) / 2n
	const whole = raw / SCALE
	const fraction = (raw % SCALE)
		.toString()
		.padStart(18, "0")
		.slice(0, maxFraction)
		.replace(/0+$/, "")
	const grouped = whole.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",")
	return fraction ? `${grouped}.${fraction}` : grouped
}

/** Which symbol the order takes in and which it pays out, from the side it sits on. */
export function legs(order: Pick<LimitOrder, "side" | "base" | "quote">): { input: string; output: string } {
	return order.side === "BID" ? { input: order.base, output: order.quote } : { input: order.quote, output: order.base }
}

/**
 * "1,500 CNGN per USDC" — the rate as the operator stated it. Rounded, not cut: the price is
 * one amount divided by the other, so a rate typed as 1374 can be stored a few 1e-18 under it.
 */
export function describeRate(order: Pick<LimitOrder, "side" | "base" | "quote" | "price">): string {
	return `${fromScaled(order.price, 6, "nearest")} ${order.quote} per ${order.base}`
}

/** How much of what the order promised is still available to a swapper. */
export function available(order: Pick<LimitOrder, "remaining" | "reserved">): bigint {
	try {
		const left = BigInt(order.remaining) - BigInt(order.reserved)
		return left > 0n ? left : 0n
	} catch {
		return 0n
	}
}

export type Tone = "" | "ok" | "warn" | "err"

/**
 * What the operator needs to know at a glance, which is not quite the status
 * column: an order can be `open` and still not be on the book, either because
 * its posting is in flight or because the last one was refused.
 */
export function statusOf(order: LimitOrder): { label: string; tone: Tone; detail?: string } {
	if (order.status === "cancelled") return { label: "Cancelled", tone: "" }
	if (order.status === "expired") return { label: "Expired", tone: "" }
	if (order.status === "filled") return { label: "Filled", tone: "ok" }
	if (order.status === "rejected") {
		return { label: "Refused", tone: "err", detail: order.lastError ?? undefined }
	}
	if (order.status === "resizing") return { label: "Resizing", tone: "warn", detail: "a fill is being settled" }
	if (order.lastError) return { label: "On the book", tone: "warn", detail: order.lastError }
	if (!order.commitment) return { label: "Posting", tone: "warn", detail: "not on the book yet" }
	return { label: "On the book", tone: "ok" }
}

/** Which way round the operator trades the book's base: buying it in, or selling it out. */
export type OrderSide = "BID" | "ASK"

/** A side as the operator reads it: a bid buys the book's base, an ask sells it. */
export function sideLabel(side: OrderSide): string {
	return side === "BID" ? "Buy" : "Sell"
}

/**
 * The order as the operator states it: a book, a side, how much of the book's base, and the rate
 * in quote per base. The book's own spelling of its symbols is carried, because that is what the
 * orderbook matches a request on.
 */
export interface LimitOrderDraft {
	book: { id: string; base: string; quote: string }
	side: OrderSide
	/**
	 * Whole tokens of what the order pays out: the quote when buying the base, the base when
	 * selling it. The same token the balance beside the field counts, and the one the
	 * orderbook's dust floor applies to.
	 */
	amount: string
	/** Quote per base: 1590 means 1590 cNGN for each USDC. */
	rate: string
	fillChain: string
	acceptedSources: string[]
}

/** A decimal figure as typed, including the half-written ones a field holds mid-keystroke. */
export const AMOUNT_PATTERN = /^[0-9]*\.?[0-9]*$/

/**
 * A decimal figure with its whole part grouped in thousands: "1590000.5" reads "1,590,000.5".
 * Only for display — the field keeps the bare figure, so a half-typed "10." stays "10." and the
 * fraction is never regrouped.
 */
export function groupThousands(value: string): string {
	const [whole, fraction] = value.split(".")
	const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",")
	return fraction === undefined ? grouped : `${grouped}.${fraction}`
}

/** A decimal string at 1e18, or null when it is not a usable number. */
export function parseAmount(value: string): bigint | null {
	const trimmed = value.trim()
	if (trimmed === "" || trimmed === "." || !AMOUNT_PATTERN.test(trimmed)) return null
	const [whole, fraction = ""] = trimmed.split(".")
	// More than 18 decimals is more than the orderbook carries, so it would be silently dropped.
	if (fraction.length > 18) return null
	return BigInt(whole || "0") * SCALE + BigInt((fraction + "0".repeat(18)).slice(0, 18))
}

/** A 1e18 value as a plain decimal string, which is what the create request carries. */
function toDecimal(value: bigint): string {
	const whole = value / SCALE
	const fraction = (value % SCALE).toString().padStart(18, "0").replace(/0+$/, "")
	return fraction ? `${whole}.${fraction}` : whole.toString()
}

/**
 * The side taken in, at 1e18, rounded up so the posted price is never better for the taker than
 * the rate the operator stated. Both sides take in: a bid takes in no less base than `amount ÷
 * rate`, an ask no less quote than `amount × rate`. Rounding up also means a positive order can
 * never ask for nothing in return.
 */
function takenIn(side: OrderSide, amount: bigint, rate: bigint): bigint {
	return side === "BID" ? (amount * SCALE + rate - 1n) / rate : (amount * rate + SCALE - 1n) / SCALE
}

/**
 * The create request a draft stands for, or null while it is not yet a whole order.
 *
 * The amount is what the order pays out, and every book is priced in quote per base, so the
 * side decides the rest: buying the base pays `amount` quote out and takes `amount ÷ rate` base
 * in; selling it pays `amount` base out and takes `amount × rate` quote in. The operator never
 * states the second amount.
 */
export function requestFrom(draft: LimitOrderDraft): CreateLimitOrderRequest | null {
	const amount = parseAmount(draft.amount)
	const rate = parseAmount(draft.rate)
	if (amount === null || rate === null || amount <= 0n || rate <= 0n) return null
	if (!draft.fillChain || draft.acceptedSources.length === 0) return null

	const { base, quote } = draft.book
	const buying = draft.side === "BID"
	return {
		fillChain: draft.fillChain,
		tokenIn: buying ? base : quote,
		amountIn: toDecimal(takenIn(draft.side, amount, rate)),
		tokenOut: buying ? quote : base,
		amountOut: toDecimal(amount),
		acceptedSources: draft.acceptedSources,
	}
}
