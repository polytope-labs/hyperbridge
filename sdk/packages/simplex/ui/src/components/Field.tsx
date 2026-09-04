import type { CSSProperties } from "react"

/** Labeled input — the wizard/operator form building block. */
export function Field(props: {
	label: string
	value: string
	onChange: (value: string) => void
	type?: "text" | "password"
	placeholder?: string
	style?: CSSProperties
	onBlur?: () => void
	required?: boolean
}) {
	return (
		<label className="field" style={props.style}>
			<span className="field-label">
				{props.label}
				{props.required ? <span className="field-required">Required</span> : null}
			</span>
			<input
				type={props.type ?? "text"}
				value={props.value}
				placeholder={props.placeholder}
				required={props.required}
				onChange={(e) => props.onChange(e.target.value)}
				onBlur={props.onBlur}
			/>
		</label>
	)
}
