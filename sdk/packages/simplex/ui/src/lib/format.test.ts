import { describe, expect, it } from "vitest"
import { formatCompactAmount } from "./format"

describe("formatCompactAmount", () => {
	it("shows an amount in full up to 100,000", () => {
		expect(formatCompactAmount(15.832)).toBe((15.832).toLocaleString(undefined, { maximumFractionDigits: 4 }))
		expect(formatCompactAmount(99_999.5)).toBe((99_999.5).toLocaleString(undefined, { maximumFractionDigits: 4 }))
	})

	it("abbreviates larger amounts so they fit a narrow figure", () => {
		expect(formatCompactAmount(123_456.78)).toBe("123.46K")
		expect(formatCompactAmount(1_234_567.89)).toBe("1.23M")
	})
})
