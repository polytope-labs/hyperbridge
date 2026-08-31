import { useState } from "react"

const ADDRESS_RE = /^0x[0-9a-fA-F]{40}$/

/** Add/remove rows of EVM addresses (allowlist editing). */
export function AddressListEditor(props: { addresses: string[]; onChange: (addresses: string[]) => void }) {
	const { addresses, onChange } = props
	const [draft, setDraft] = useState("")
	const [error, setError] = useState<string>()

	const add = () => {
		const value = draft.trim()
		if (!ADDRESS_RE.test(value)) {
			setError("Enter a valid EVM address (0x + 40 hex chars)")
			return
		}
		if (addresses.some((a) => a.toLowerCase() === value.toLowerCase())) {
			setError("Address is already in the list")
			return
		}
		setError(undefined)
		setDraft("")
		onChange([...addresses, value])
	}

	return (
		<div>
			{addresses.map((address) => (
				<div className="row" key={address} style={{ marginBottom: "0.35rem" }}>
					<span className="mono" style={{ flex: 1 }}>
						{address}
					</span>
					<button type="button" onClick={() => onChange(addresses.filter((a) => a !== address))}>
						✕
					</button>
				</div>
			))}
			<div className="row">
				<input
					type="text"
					style={{ flex: 1 }}
					placeholder="0x…"
					value={draft}
					onChange={(e) => {
						setDraft(e.target.value)
						setError(undefined)
					}}
					onKeyDown={(e) => {
						if (e.key === "Enter") {
							e.preventDefault()
							add()
						}
					}}
				/>
				<button type="button" onClick={add} disabled={!draft.trim()}>
					+ Add address
				</button>
			</div>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
