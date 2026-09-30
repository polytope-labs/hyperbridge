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
		case "version-skew":
			return (
				<div className="connection-error" role="alert">
					<span className="eyebrow">Restart required</span>
					<h1>Simplex versions do not match</h1>
					<p>
						The desktop app is version {state.desktopVersion}, but the running solver is version{" "}
						{state.solverVersion}. The dashboard is blocked so a newer UI cannot drive an older solver.
					</p>
					<p>Choose “Restart with bundled solver” from the Simplex tray or application menu.</p>
					<button type="button" onClick={() => void refresh()}>
						Check again
					</button>
				</div>
			)
		case "operator":
			return <Operator status={state.status} refresh={refresh} />
		case "setup":
			return <Wizard defaults={state.defaults} />
	}
}
