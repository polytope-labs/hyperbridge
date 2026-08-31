/** Pill-style tab row used for page tabs, signer types and pricing sources. */
export function PillTabs<T extends string>(props: {
	options: ReadonlyArray<{ value: T; label: string }>
	value: T
	onChange: (value: T) => void
}) {
	return (
		<div className="steps">
			{props.options.map((option) => (
				<button
					key={option.value}
					type="button"
					className={`step ${props.value === option.value ? "active" : ""}`}
					style={{ cursor: "pointer" }}
					onClick={() => props.onChange(option.value)}
				>
					{option.label}
				</button>
			))}
		</div>
	)
}
