import { useLayoutEffect, useReducer, useRef, type ChangeEvent, type ClipboardEvent } from "react"

/** Characters that survive grouping, and so anchor the caret across a reformat. */
const SIGNIFICANT = /[0-9.]/

/** Groups the integer part only, so a long decimal keeps every digit and a trailing point stays. */
export function groupThousands(amount: string): string {
	const [integer, ...fraction] = amount.split(".")
	const grouped = integer.replace(/\B(?=(\d{3})+(?!\d))/g, ",")
	return fraction.length > 0 ? `${grouped}.${fraction.join("")}` : grouped
}

/**
 * Reads text the operator typed or pasted as a plain decimal, or null when it is not a number.
 *
 * A comma may be a decimal mark or a thousands separator, and dropping it would turn "1,5" into
 * 15. It is read as a separator only where it cannot be a decimal mark: beside a point, or in
 * groups of exactly three digits after a group that does not start with 0, as in 1,000. Grouping
 * never starts with a 0 group, so 0,500 is half a unit.
 */
function readInserted(text: string): string | null {
	const compact = text.replace(/\s/g, "")
	if (/[^0-9.,]/.test(compact)) return null
	const lastComma = compact.lastIndexOf(",")
	const lastPoint = compact.lastIndexOf(".")
	if (lastComma === -1) return compact
	if (lastPoint !== -1) {
		// With both marks, the later one is the decimal mark and the other one groups thousands.
		return lastComma > lastPoint ? compact.replaceAll(".", "").replace(",", ".") : compact.replaceAll(",", "")
	}
	if (/^[1-9]\d{0,2}(,\d{3})+$/.test(compact)) return compact.replaceAll(",", "")
	return compact.indexOf(",") === lastComma ? compact.replace(",", ".") : null
}

/**
 * The amount without the zeros and the point that change nothing: 1.000 is 1 and 0.500 is 0.5.
 * Shown and sent this way, an amount with a comma typed as its decimal mark cannot pass for a
 * thousand.
 */
export function trimAmount(amount: string): string {
	if (!amount.includes(".")) return amount
	const trimmed = amount.replace(/0+$/, "").replace(/\.$/, "")
	return trimmed.startsWith(".") ? `0${trimmed}` : trimmed || "0"
}

/**
 * The plain amount after one edit of the grouped text, and the caret as a count of the plain
 * characters before it. An edit that does not leave a number is refused: the amount stays as it
 * was.
 *
 * `shown` is the grouped text before the edit, `typed` the field's text after it, and `caret` the
 * caret in `typed`. What follows the caret was in the field before, and its commas are the field's
 * own grouping. What the edit put in front of the caret is read as the operator's.
 */
export function applyAmountEdit(
	shown: string,
	typed: string,
	caret: number,
	inputType?: string,
): { value: string; caret: number } {
	const before = shown.replaceAll(",", "")
	const tail = typed.slice(caret)
	// When the text after the caret is not the end of what was shown, the whole value was replaced.
	const kept = shown.endsWith(tail) ? shown.length - tail.length : 0
	let start = 0
	while (start < caret && start < kept && typed[start] === shown[start]) start += 1
	const head = typed.slice(0, start).replaceAll(",", "")
	const rest = shown.endsWith(tail) ? tail.replaceAll(",", "") : ""
	const refused = { value: before, caret: head.length }

	const inserted = readInserted(shown.endsWith(tail) ? typed.slice(start, caret) : typed)
	if (inserted === null) return refused
	let value = head + inserted + rest
	let position = head.length + inserted.length
	// Deleting a separator alone changes nothing, so the key also takes the digit it was aimed at.
	if (inserted === "" && value === before && typed.length < shown.length) {
		if (inputType === "deleteContentForward") {
			value = value.slice(0, position) + value.slice(position + 1)
		} else if (position > 0) {
			value = value.slice(0, position - 1) + value.slice(position)
			position -= 1
		}
	}
	if (!/^\d*\.?\d*$/.test(value)) return refused

	const trimmed = value.replace(/^0+(?=\d)/, "")
	return { value: trimmed, caret: Math.max(0, position - (value.length - trimmed.length)) }
}

/**
 * The plain amount after `pasted` replaces the characters from `start` to `end` of the grouped
 * text, and the caret as a count of the plain characters before it.
 *
 * A paste is not read through {@link applyAmountEdit}. That sees only the text before and after,
 * and takes a comma the two share for the field's own grouping. With 1,000 selected and 1,5
 * pasted over it, it would drop the pasted comma and leave 15.
 */
export function applyAmountPaste(
	shown: string,
	start: number,
	end: number,
	pasted: string,
): { value: string; caret: number } {
	const head = shown.slice(0, start).replaceAll(",", "")
	const rest = shown.slice(end).replaceAll(",", "")
	const refused = { value: shown.replaceAll(",", ""), caret: head.length }
	// A clipboard with no text on it, such as an image, pastes nothing. It must not clear the selection.
	const inserted = pasted === "" ? null : readInserted(pasted)
	if (inserted === null) return refused
	const value = head + inserted + rest
	if (!/^\d*\.?\d*$/.test(value)) return refused

	const trimmed = value.replace(/^0+(?=\d)/, "")
	const position = head.length + inserted.length
	return { value: trimmed, caret: Math.max(0, position - (value.length - trimmed.length)) }
}

function caretAfterSignificant(value: string, target: number): number {
	if (target <= 0) return 0
	let seen = 0
	for (let i = 0; i < value.length; i += 1) {
		if (SIGNIFICANT.test(value[i])) {
			seen += 1
			if (seen === target) return i + 1
		}
	}
	return value.length
}

/**
 * An amount field that shows thousands separators while reporting a plain value.
 *
 * A separator shifts every character to its right, so a plain controlled input throws the caret
 * to the end as soon as one appears. The caret is kept by counting digits instead of characters,
 * and put back after the value commits.
 */
export function AmountInput(props: {
	id: string
	value: string
	onValueChange: (raw: string) => void
	invalid?: boolean
}) {
	const inputRef = useRef<HTMLInputElement>(null)
	const pendingCaret = useRef<number | null>(null)
	// A refused edit leaves the value as it was, and the caret still has to be put back.
	const [edits, countEdit] = useReducer((count: number) => count + 1, 0)
	const display = groupThousands(props.value)

	// Runs after every edit and every new value, which is when the caret has to be put back.
	useLayoutEffect(() => {
		const target = pendingCaret.current
		pendingCaret.current = null
		const input = inputRef.current
		if (target === null || !input) return
		const caret = caretAfterSignificant(input.value, target)
		input.setSelectionRange(caret, caret)
	}, [display, edits])

	const commit = (edit: { value: string; caret: number }) => {
		pendingCaret.current = edit.caret
		countEdit()
		if (edit.value !== props.value) props.onValueChange(edit.value)
	}

	const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
		const { selectionStart, value: typed } = event.target
		const inputType = (event.nativeEvent as Partial<InputEvent>).inputType
		commit(applyAmountEdit(display, typed, selectionStart ?? typed.length, inputType))
	}

	/** A paste names its own text and the range it replaces, so neither has to be worked out. */
	const handlePaste = (event: ClipboardEvent<HTMLInputElement>) => {
		const input = event.currentTarget
		event.preventDefault()
		const start = input.selectionStart ?? display.length
		commit(applyAmountPaste(display, start, input.selectionEnd ?? start, event.clipboardData.getData("text")))
	}

	return (
		<input
			ref={inputRef}
			id={props.id}
			type="text"
			inputMode="decimal"
			autoComplete="off"
			placeholder="0.00"
			value={display}
			aria-invalid={props.invalid || undefined}
			onChange={handleChange}
			onPaste={handlePaste}
		/>
	)
}
