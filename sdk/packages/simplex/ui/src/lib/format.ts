/**
 * Formats a raw on-chain amount (decimal string) with `decimals`, trimming to at
 * most `maxFraction` fractional digits and grouping thousands. Falls back to the
 * raw integer when decimals are unknown, so a row never shows the wrong scale.
 */
/**
 * Fraction digits that suit the size of the number: 19,990.9995 reads as 19,991
 * while 0.9995 keeps its four places. Pass `maxFraction` to override (e.g. the
 * token's full precision for a tooltip).
 */
function fractionDigitsFor(whole: string): number {
	const magnitude = whole.replace(/^0+/, "").length
	if (magnitude >= 5) return 0
	if (magnitude >= 3) return 2
	return 4
}

export function formatTokenAmount(raw: string, decimals: number | null, maxFraction?: number): string {
	if (!/^\d+$/.test(raw)) return raw
	if (decimals === null) return groupThousands(raw)
	if (decimals === 0) return groupThousands(raw)
	const padded = raw.padStart(decimals + 1, "0")
	const digits = maxFraction ?? fractionDigitsFor(padded.slice(0, padded.length - decimals))
	// Round half up at the chosen precision rather than truncating, so 19,990.9995 becomes 19,991.
	const keep = Math.min(digits, decimals)
	const drop = decimals - keep
	let scaled = BigInt(raw)
	if (drop > 0) {
		const unit = 10n ** BigInt(drop)
		scaled = (scaled + unit / 2n) / unit
	}
	const text = scaled.toString().padStart(keep + 1, "0")
	const whole = text.slice(0, text.length - keep)
	const fraction = keep > 0 ? text.slice(text.length - keep).replace(/0+$/, "") : ""
	// A dust amount that rounds to nothing still deserves a sign of life.
	if (fraction === "" && whole === "0" && raw !== "0") return `<0.${"0".repeat(Math.max(digits - 1, 0))}1`
	return fraction ? `${groupThousands(whole)}.${fraction}` : groupThousands(whole)
}

/** Same rounding as `formatTokenAmount`, for an amount already written as a decimal string ("178.21999"). */
export function formatDecimalAmount(value: string, maxFraction?: number): string {
	const match = /^(\d+)(?:\.(\d+))?$/.exec(value.trim())
	if (!match) return value
	const fraction = match[2] ?? ""
	return formatTokenAmount(match[1] + fraction, fraction.length, maxFraction)
}

function groupThousands(integer: string): string {
	return integer.replace(/\B(?=(\d{3})+(?!\d))/g, ",")
}

/** A number already in display units: grouped thousands, at most four decimals. */
export function formatAmount(value: number): string {
	return value.toLocaleString(undefined, { maximumFractionDigits: 4 })
}

/** "09:05 AM" in the viewer's locale. */
export function formatClockTime(ts: number): string {
	return new Date(ts).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })
}

/** "5 Sept 2026" in the viewer's locale. */
export function formatDate(ts: number): string {
	return new Date(ts).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })
}

/**
 * A referrer tag for display. Tags are 32 bytes: apps write their name as
 * left-aligned ASCII ("HyperFX", "pawasave"), integrations may write an address
 * in the low 20 bytes. Printable text wins; otherwise the address or short hex.
 */
export function describeReferrer(tag: string): string {
	const hex = tag.replace(/^0x/, "")
	if (hex.length !== 64) return shortAddress(tag)
	const trimmed = hex.replace(/(00)+$/, "")
	if (trimmed.length > 0 && trimmed.length % 2 === 0) {
		const bytes = trimmed.match(/../g) ?? []
		const text = bytes.map((byte) => String.fromCharCode(Number.parseInt(byte, 16))).join("")
		if (/^[\x20-\x7e]+$/.test(text)) return text
	}
	return /^0{24}/.test(hex) ? shortAddress(`0x${hex.slice(24)}`) : shortAddress(tag)
}

/** Milliseconds since epoch for a SQLite-style "YYYY-MM-DD HH:MM:SS" UTC timestamp. */
export function sqliteUtcToMs(value: string): number {
	return Date.parse(`${value.replace(" ", "T")}Z`)
}

/** "0xeb9c…0e8a" */
export function shortAddress(value: string, head = 6, tail = 4): string {
	return value.length > head + tail + 2 ? `${value.slice(0, head)}…${value.slice(-tail)}` : value
}
