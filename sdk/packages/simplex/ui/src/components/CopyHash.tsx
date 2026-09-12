import { type ReactNode, useState } from "react"
import { CheckIcon, CopyIcon } from "./InterfaceIcons"

/** Truncated hash with a click-to-copy button for the full value. */
export function CopyHash(props: { value: string; chars?: number; copyLabel?: string; children?: ReactNode }) {
	const { value, chars = 10, copyLabel = "Copy value", children } = props
	const [copied, setCopied] = useState(false)
	const copy = () => {
		void navigator.clipboard
			.writeText(value)
			.then(() => {
				setCopied(true)
				window.setTimeout(() => setCopied(false), 1200)
			})
			.catch(() => setCopied(false))
	}

	return (
		<span className="copy-value">
			<span className="copy-value-text mono" title={value}>
				{children ?? (value.length > chars ? `${value.slice(0, chars)}…` : value)}
			</span>
			<button
				type="button"
				className="copy-value-button"
				aria-label={copied ? "Copied" : copyLabel}
				title={copied ? "Copied" : copyLabel}
				onClick={copy}
			>
				{copied ? <CheckIcon aria-hidden="true" /> : <CopyIcon aria-hidden="true" />}
			</button>
		</span>
	)
}
