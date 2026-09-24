import { type ReactNode, useEffect, useRef, useState } from "react"
import { ChainLogo } from "./ChainLogo"
import { CheckIcon, ChevronDownIcon } from "./InterfaceIcons"

/**
 * Pick several chains from a menu that looks like {@link AppSelect}'s. Radix Select is single
 * choice only, so the menu is a plain panel of checkboxes under the trigger. It closes on Escape
 * or a click outside, and is not portalled, so it works inside a sheet's focus trap.
 */
export function ChainMultiSelect(props: {
	ariaLabel: string
	/** `leading` replaces the chain's logo on the row; `trailing` sits at its right edge. */
	options: Array<{ value: string; label: string; leading?: ReactNode; trailing?: ReactNode }>
	value: string[]
	onValueChange: (value: string[]) => void
}) {
	const { ariaLabel, options, value, onValueChange } = props
	const [open, setOpen] = useState(false)
	const root = useRef<HTMLDivElement>(null)
	const trigger = useRef<HTMLButtonElement>(null)

	useEffect(() => {
		if (!open) return
		const close = (event: MouseEvent) => {
			if (!root.current?.contains(event.target as Node)) setOpen(false)
		}
		document.addEventListener("mousedown", close)
		return () => document.removeEventListener("mousedown", close)
	}, [open])

	const chosen = options.filter((option) => value.includes(option.value))
	const summary =
		chosen.length === options.length
			? "All networks"
			: chosen.length === 0
				? "No networks"
				: chosen.length <= 2
					? chosen.map((option) => option.label).join(", ")
					: `${chosen.length} networks`

	const toggle = (option: string) =>
		onValueChange(value.includes(option) ? value.filter((entry) => entry !== option) : [...value, option])

	return (
		<div
			className="chain-multiselect"
			ref={root}
			onKeyDown={(event) => {
				if (event.key !== "Escape" || !open) return
				// The sheet closes on Escape too; this one belongs to the menu.
				event.stopPropagation()
				setOpen(false)
				trigger.current?.focus()
			}}
		>
			<button
				type="button"
				ref={trigger}
				className="app-select-trigger"
				aria-label={ariaLabel}
				aria-haspopup="listbox"
				aria-expanded={open}
				// Radix sets this on its own trigger; the shared styles key the open look and the
				// chevron's turn off it.
				data-state={open ? "open" : "closed"}
				onClick={() => setOpen((current) => !current)}
			>
				<span className="app-select-current">
					{chosen.length > 0 ? (
						<span className="chain-multiselect-logos" aria-hidden="true">
							{chosen.map((option) => (
								<ChainLogo key={option.value} label={option.label} />
							))}
						</span>
					) : null}
					<span className="app-select-value">{summary}</span>
				</span>
				<span className="app-select-chevron">
					<ChevronDownIcon aria-hidden="true" />
				</span>
			</button>
			{open ? (
				<div className="app-select-content chain-multiselect-content" role="listbox" aria-multiselectable="true">
					<div className="app-select-viewport">
						{options.map((option) => {
							const checked = value.includes(option.value)
							return (
								<label
									key={option.value}
									className="app-select-item"
									data-state={checked ? "checked" : "unchecked"}
								>
									<input
										type="checkbox"
										className="chain-multiselect-check"
										checked={checked}
										onChange={() => toggle(option.value)}
									/>
									{option.leading ?? <ChainLogo label={option.label} />}
									<span className="chain-multiselect-label">{option.label}</span>
									{option.trailing === undefined ? null : (
										<span className="app-select-item-trailing">{option.trailing}</span>
									)}
									{/* Always laid out, so trailing values line up whether or not a row is ticked. */}
									<span className="app-select-indicator" data-hidden={checked ? undefined : true}>
										<CheckIcon aria-hidden="true" />
									</span>
								</label>
							)
						})}
					</div>
				</div>
			) : null}
		</div>
	)
}
