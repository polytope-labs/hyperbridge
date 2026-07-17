import { useCallback, useEffect, useState } from "react"
import { api } from "./api"
import { Operator } from "./operator/Operator"
import type { SetupDefaults, Status } from "./types"
import { Wizard } from "./wizard/Wizard"

export function App() {
	const [status, setStatus] = useState<Status>()
	const [defaults, setDefaults] = useState<SetupDefaults>()
	const [error, setError] = useState<string>()

	const refresh = useCallback(async () => {
		try {
			const next = await api.get<Status>("/api/status")
			setStatus(next)
			setError(undefined)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])

	useEffect(() => {
		refresh()
	}, [refresh])

	useEffect(() => {
		if (status?.mode !== "init") return
		api.get<SetupDefaults>("/api/setup/defaults").then(setDefaults).catch((err) => setError(String(err)))
	}, [status?.mode])

	if (error) {
		return (
			<div className="card">
				<p className="error">Cannot reach the simplex process: {error}</p>
				<button type="button" onClick={refresh}>
					Retry
				</button>
			</div>
		)
	}
	if (!status) return <p className="hint">Connecting…</p>

	if (status.mode === "operator") {
		return <Operator status={status} refresh={refresh} />
	}
	if (!defaults) return <p className="hint">Loading setup…</p>
	return <Wizard defaults={defaults} />
}
