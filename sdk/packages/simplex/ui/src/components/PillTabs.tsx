/** Pill-style tab row used for page tabs, signer types and pricing sources. */
export function PillTabs<T extends string>(props: {
	/** `count` is how many items a tab holds, shown beside its label. */
	options: ReadonlyArray<{ value: T; label: string; count?: number }>
	value: T
	onChange: (value: T) => void
	className?: string
	ariaLabel?: string
}) {
	return (
		<div
			className={`steps${props.className ? ` ${props.className}` : ""}`}
			role="group"
			aria-label={props.ariaLabel}
		>
			{props.options.map((option) => (
				<button
					key={option.value}
					type="button"
					className={`step ${props.value === option.value ? "active" : ""}`}
					aria-pressed={props.value === option.value}
					style={{ cursor: "pointer" }}
					onClick={() => props.onChange(option.value)}
				>
					{option.label}
					{option.count === undefined ? null : <span className="step-count">{option.count}</span>}
				</button>
			))}
		</div>
	)
}
