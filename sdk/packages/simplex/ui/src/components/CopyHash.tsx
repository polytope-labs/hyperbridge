import { useState } from "react"

/** Truncated hash with a click-to-copy button for the full value. */
export function CopyHash(props: { value: string; chars?: number }) {
	const [copied, setCopied] = useState(false)
	const chars = props.chars ?? 10

	const copy = async () => {
		try {
			await navigator.clipboard.writeText(props.value)
			setCopied(true)
			setTimeout(() => setCopied(false), 1200)
		} catch {
			// clipboard unavailable — the title attribute still exposes the full value
		}
	}

	return (
		<span className="row" style={{ gap: "0.3rem", display: "inline-flex" }}>
			<span className="mono" title={props.value}>
				{props.value.slice(0, chars)}…
			</span>
			<button
				type="button"
				title="Copy full hash"
				onClick={copy}
				style={{ padding: "0 0.4rem", fontSize: "0.75rem", lineHeight: "1.4" }}
			>
				{copied ? "✓" : "⧉"}
			</button>
		</span>
	)
}
