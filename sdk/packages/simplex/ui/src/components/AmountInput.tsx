import { useLayoutEffect, useReducer, useRef, type ChangeEvent } from "react"

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
 * 15. It is read as a separator only where it cannot be a decimal mark: beside a point, or before
 * exactly three digits, as in 1,000.
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
	if (/^\d{1,3}(,\d{3})+$/.test(compact)) return compact.replaceAll(",", "")
	return compact.indexOf(",") === lastComma ? compact.replace(",", ".") : null
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

	const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
		const { selectionStart, value: typed } = event.target
		const inputType = (event.nativeEvent as Partial<InputEvent>).inputType
		const edit = applyAmountEdit(display, typed, selectionStart ?? typed.length, inputType)
		pendingCaret.current = edit.caret
		countEdit()
		if (edit.value !== props.value) props.onValueChange(edit.value)
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
		/>
	)
}
