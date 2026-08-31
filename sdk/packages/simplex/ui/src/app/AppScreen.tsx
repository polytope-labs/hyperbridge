import { Operator } from "../operator/Operator"
import { Wizard } from "../wizard/Wizard"
import type { AppBootstrapState } from "./useAppBootstrap"

export function AppScreen(props: { state: AppBootstrapState; refresh: () => Promise<void> }) {
	const { state, refresh } = props
	switch (state.kind) {
		case "error":
			return (
				<div className="connection-error" role="alert">
					<span className="eyebrow">Connection unavailable</span>
					<h1>Simplex could not be reached</h1>
					<p>{state.message}</p>
					<button type="button" onClick={() => void refresh()}>
						Retry connection
					</button>
				</div>
			)
		case "connecting":
			return <p className="hint">Connecting…</p>
		case "loading-setup":
			return <p className="hint">Loading setup…</p>
		case "operator":
			return <Operator status={state.status} refresh={() => void refresh()} />
		case "setup":
			return <Wizard defaults={state.defaults} />
	}
}
