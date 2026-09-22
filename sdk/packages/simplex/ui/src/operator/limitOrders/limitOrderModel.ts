import type { LimitOrder } from "../../types"

/** The orderbook keeps every amount and price at 1e18, whatever the token's own decimals. */
const SCALE = 10n ** 18n

/**
 * A 1e18 figure as whole tokens, trimmed of trailing zeros.
 *
 * The operator states amounts in whole tokens and reads them back the same way:
 * what the orderbook normalises to is not their problem.
 */
export function fromScaled(value: string, maxFraction = 6): string {
	let raw: bigint
	try {
		raw = BigInt(value)
	} catch {
		return value
	}
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

/** "1,500 CNGN per USDC" — the rate as the operator stated it. */
export function describeRate(order: Pick<LimitOrder, "side" | "base" | "quote" | "price">): string {
	return `${fromScaled(order.price)} ${order.quote} per ${order.base}`
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
