import { describe, expect, it } from "vitest"
import { applyAmountEdit, groupThousands, trimAmount } from "./AmountInput"

/** Types or pastes `text` at the end of a field showing `shown`. */
const append = (shown: string, text: string) => applyAmountEdit(shown, shown + text, shown.length + text.length).value

describe("amount edits", () => {
	it("keeps the field's own separators out of the amount as digits are typed", () => {
		expect(append("1,234", "5")).toBe("12345")
		expect(groupThousands("12345")).toBe("12,345")
		// A digit typed after the first one, in front of the separator.
		expect(applyAmountEdit("1,234", "19,234", 2)).toEqual({ value: "19234", caret: 2 })
	})

	it("reads a comma as the decimal mark unless it can only be a separator", () => {
		expect(append("", "1,5")).toBe("1.5")
		expect(append("", "0,25")).toBe("0.25")
		expect(append("1", ",")).toBe("1.")
		expect(append("", "1,000")).toBe("1000")
		expect(append("", "12,345,678")).toBe("12345678")
		expect(append("", "1,000.50")).toBe("1000.50")
		expect(append("", "1.000,50")).toBe("1000.50")
		expect(append("", "1 000,50")).toBe("1000.50")
	})

	it("reads a group that starts with 0 as decimals, since grouping never starts that way", () => {
		expect(trimAmount(append("", "0,500"))).toBe("0.5")
		expect(append("", "0,125")).toBe("0.125")
		expect(append("", "100,000")).toBe("100000")
	})

	it("reads a comma typed after digits as the decimal mark", () => {
		// Typed one key at a time, 1,000 is 1: reading it as a thousand would also read 1,5 as 15.
		const typed = ["1", ",", "0", "0", "0"].reduce((shown, key) => groupThousands(append(shown, key)), "")
		expect(typed).toBe("1.000")
		expect(trimAmount(typed)).toBe("1")
	})

	it("drops the zeros and the point that change nothing", () => {
		expect(trimAmount("1.000")).toBe("1")
		expect(trimAmount("1.50")).toBe("1.5")
		expect(trimAmount("1000")).toBe("1000")
		expect(trimAmount(".5")).toBe("0.5")
		expect(trimAmount("0.")).toBe("0")
		expect(trimAmount("10.050")).toBe("10.05")
	})

	it("refuses text that is not a number, and leaves the amount as it was", () => {
		expect(append("", "1e3")).toBe("")
		expect(append("", "-5")).toBe("")
		expect(append("", "1.2.3")).toBe("")
		expect(append("", "1,2,3")).toBe("")
		expect(append("1.5", ",")).toBe("1.5")
		expect(applyAmountEdit("1,234", "1,2x34", 4)).toEqual({ value: "1234", caret: 2 })
	})

	it("takes the digit beside a separator when the separator alone is deleted", () => {
		// Backspace with the caret after the comma removes the 1.
		expect(applyAmountEdit("1,234", "1234", 1, "deleteContentBackward")).toEqual({ value: "234", caret: 0 })
		// Delete with the caret before the comma removes the 2.
		expect(applyAmountEdit("1,234", "1234", 1, "deleteContentForward")).toEqual({ value: "134", caret: 1 })
	})

	it("replaces the whole amount when everything was selected", () => {
		expect(applyAmountEdit("1,000", "25", 2)).toEqual({ value: "25", caret: 2 })
	})

	it("drops zeros in front of the amount", () => {
		expect(append("", "0001000")).toBe("1000")
		expect(append("0", "5")).toBe("5")
		expect(append("0", ".5")).toBe("0.5")
	})
})
