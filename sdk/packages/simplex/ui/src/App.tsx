import { Toaster } from "sonner"
import { AppScreen } from "./app/AppScreen"
import { useAppBootstrap } from "./app/useAppBootstrap"
import { ScreenErrorBoundary } from "./components/ScreenErrorBoundary"

export function App() {
	const { state, refresh } = useAppBootstrap()

	return (
		<>
			<div className="app-shell">
				<HeaderGradient />
				<main className={`app-container ${state.kind === "operator" ? "operator-container" : ""}`}>
					<ScreenErrorBoundary>
						<AppScreen state={state} refresh={refresh} />
					</ScreenErrorBoundary>
				</main>
			</div>
			<Toaster className="simplex-toaster" position="top-right" theme="dark" closeButton richColors />
		</>
	)
}

function HeaderGradient() {
	return <div className="header-gradient bg-gradient-brand-animated" />
}
