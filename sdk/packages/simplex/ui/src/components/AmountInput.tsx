import { useLayoutEffect, useRef, type ChangeEvent } from "react"

/** Characters that survive grouping, and so anchor the caret across a reformat. */
const SIGNIFICANT = /[0-9.]/

/** Groups the integer part only, so a long decimal keeps every digit and a trailing point stays. */
export function groupThousands(amount: string): string {
	const [integer, ...fraction] = amount.split(".")
	const grouped = integer.replace(/\B(?=(\d{3})+(?!\d))/g, ",")
	return fraction.length > 0 ? `${grouped}.${fraction.join("")}` : grouped
}

/** What was typed, as a plain decimal: digits and at most one point. */
function ungroup(typed: string): string {
	const [integer, ...fraction] = typed.replace(/[^0-9.]/g, "").split(".")
	return fraction.length > 0 ? `${integer}.${fraction.join("")}` : integer
}

function countSignificantBefore(value: string, caret: number): number {
	let count = 0
	for (let i = 0; i < caret && i < value.length; i += 1) {
		if (SIGNIFICANT.test(value[i])) count += 1
	}
	return count
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
	placeholder?: string
	invalid?: boolean
}) {
	const inputRef = useRef<HTMLInputElement>(null)
	const pendingCaret = useRef<number | null>(null)
	const display = groupThousands(props.value)

	// Runs when the shown value changes, which is when the caret has to be put back.
	useLayoutEffect(() => {
		const target = pendingCaret.current
		pendingCaret.current = null
		const input = inputRef.current
		if (target === null || !input) return
		const caret = caretAfterSignificant(input.value, target)
		input.setSelectionRange(caret, caret)
	}, [display])

	const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
		const { selectionStart, value: typed } = event.target
		pendingCaret.current = countSignificantBefore(typed, selectionStart ?? typed.length)
		props.onValueChange(ungroup(typed))
	}

	return (
		<input
			ref={inputRef}
			id={props.id}
			type="text"
			inputMode="decimal"
			autoComplete="off"
			placeholder={props.placeholder ?? "0.00"}
			value={display}
			aria-invalid={props.invalid || undefined}
			onChange={handleChange}
		/>
	)
}
