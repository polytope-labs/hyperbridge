import { describe, expect, it } from "vitest"
import { applyAmountEdit, applyAmountPaste, groupThousands, isAmbiguousAmount, trimAmount } from "./AmountInput"

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
		expect(append("", "12,345,678")).toBe("12345678")
		expect(append("", "1,000.50")).toBe("1000.50")
		expect(append("", "1.000,50")).toBe("1000.50")
		expect(append("", "1 000,50")).toBe("1000.50")
	})

	it("refuses text that arrives whole and reads two ways, a thousandfold apart", () => {
		// 1,500 is fifteen hundred to one reader and one and a half to another.
		for (const text of ["1,500", "12,345", "999,999", "1,000", " 1,500 "]) {
			expect(isAmbiguousAmount(text)).toBe(true)
			expect(applyAmountPaste("", 0, 0, text)).toEqual({ value: "", caret: 0 })
			expect(append("", text)).toBe("")
		}
		// Pasted back over itself, the field's own 1,500 stays fifteen hundred.
		expect(applyAmountPaste("1,500", 0, 5, "1,500")).toEqual({ value: "1500", caret: 0 })
	})

	it("reads a lone comma as the decimal mark where it can be nothing else", () => {
		// Grouping never starts with a 0 group, and never has two or four digits after the comma.
		expect(trimAmount(append("", "0,500"))).toBe("0.5")
		expect(append("", "0,125")).toBe("0.125")
		expect(append("", "1,50")).toBe("1.50")
		expect(append("", "1,5000")).toBe("1.5000")
		expect(append("", "1234,567")).toBe("1234.567")
		for (const text of ["0,500", "1,5", "1,234,567", "1,500.00", "1500"]) expect(isAmbiguousAmount(text)).toBe(false)
	})

	it("reads several commas as separators only in groups of three", () => {
		expect(applyAmountPaste("", 0, 0, "1,234,567")).toEqual({ value: "1234567", caret: 7 })
		expect(applyAmountPaste("", 0, 0, "1,2,3")).toEqual({ value: "", caret: 0 })
		expect(applyAmountPaste("", 0, 0, "0,500,000")).toEqual({ value: "", caret: 0 })
	})

	it("reads text an event names by where the event put it, not by what it shares with the old amount", () => {
		// Dropped or put in by a keyboard over a selected 1,000. Matched against the old text, "1," is
		// shared, the comma is taken for the field's own, and 1,5 becomes 15.
		expect(applyAmountEdit("1,000", "1,5", 3, "insertText", "1,5")).toEqual({ value: "1.5", caret: 3 })
		expect(applyAmountEdit("1,000", "1,5", 3)).toEqual({ value: "15", caret: 2 })
		// The same for text that reads two ways: it is refused, where the match would enter 1500.
		expect(applyAmountEdit("1,000", "1,500", 5, "insertFromDrop", "1,500")).toEqual({ value: "1000", caret: 0 })
		// Into the middle of an amount, between what was there before and after.
		expect(applyAmountEdit("1,234", "1,99234", 4, "insertText", "99")).toEqual({ value: "199234", caret: 3 })
		// A single key is located as before.
		expect(applyAmountEdit("1,234", "19,234", 2, "insertText", "9")).toEqual({ value: "19234", caret: 2 })
	})

	it("reads a comma typed after digits as the decimal mark", () => {
		// Typed one key at a time, 1,000 is 1: reading it as a thousand would also read 1,5 as 15.
		const typed = ["1", ",", "0", "0", "0"].reduce((shown, key) => groupThousands(append(shown, key)), "")
		expect(typed).toBe("1.000")
		expect(trimAmount(typed)).toBe("1")
	})

	it("reads a paste over a selection by the pasted text, not by what it shares with the old amount", () => {
		// Everything selected. Read as an edit, the shared "1," would hide the pasted comma and leave 15.
		expect(applyAmountPaste("1,000", 0, 5, "1,5")).toEqual({ value: "1.5", caret: 3 })
		expect(applyAmountPaste("12,000", 0, 6, "12,5")).toEqual({ value: "12.5", caret: 4 })
		expect(applyAmountPaste("1,234", 0, 5, "1,2")).toEqual({ value: "1.2", caret: 3 })
		// Into an empty field, and into the middle of an amount.
		expect(applyAmountPaste("", 0, 0, "1,000.50")).toEqual({ value: "1000.50", caret: 7 })
		expect(applyAmountPaste("1,234", 1, 5, "9")).toEqual({ value: "19", caret: 2 })
		// Text that is not a number, or that leaves two points, changes nothing.
		expect(applyAmountPaste("1.5", 0, 0, "abc")).toEqual({ value: "1.5", caret: 0 })
		// A clipboard with no text on it leaves a selected amount as it is.
		expect(applyAmountPaste("1,000", 0, 5, "")).toEqual({ value: "1000", caret: 0 })
		expect(applyAmountPaste("1.5", 3, 3, "0,5")).toEqual({ value: "1.5", caret: 3 })
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
