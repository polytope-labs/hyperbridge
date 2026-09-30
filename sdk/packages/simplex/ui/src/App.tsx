import { Toaster } from "sonner"
import { AppScreen } from "./app/AppScreen"
import { useAppBootstrap } from "./app/useAppBootstrap"
import { ScreenErrorBoundary } from "./components/ScreenErrorBoundary"
import { InstallAppProvider } from "./components/InstallAppButton"
import { DesktopAccess } from "./app/DesktopAccess"

export function App() {
	return (
		<DesktopAccess>
			<AppContent />
		</DesktopAccess>
	)
}

function AppContent() {
	const { state, refresh } = useAppBootstrap()

	return (
		<InstallAppProvider>
			<div className={`app-shell ${state.kind === "operator" ? "app-shell-operator" : ""}`}>
				<HeaderGradient />
				<main className={`app-container ${state.kind === "operator" ? "operator-container" : ""}`}>
					<ScreenErrorBoundary>
						<AppScreen state={state} refresh={refresh} />
					</ScreenErrorBoundary>
				</main>
			</div>
			<Toaster className="simplex-toaster" position="top-right" theme="dark" closeButton richColors />
		</InstallAppProvider>
	)
}

function HeaderGradient() {
	return <div className="header-gradient bg-gradient-brand-animated" />
}
