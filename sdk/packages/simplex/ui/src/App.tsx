import { Toaster } from "sonner"
import { AppScreen } from "./app/AppScreen"
import { useAppBootstrap } from "./app/useAppBootstrap"
import { ScreenErrorBoundary } from "./components/ScreenErrorBoundary"
import { InstallAppProvider } from "./components/InstallAppButton"
import { DesktopAccess } from "./app/DesktopAccess"
import { DesktopUpdateProvider, UpdatingScreen, useDesktopUpdate } from "./app/DesktopUpdate"

export function App() {
	return (
		<DesktopAccess>
			<DesktopUpdateProvider>
				<AppContent />
			</DesktopUpdateProvider>
		</DesktopAccess>
	)
}

function AppContent() {
	const { state, refresh } = useAppBootstrap()
	const { installing } = useDesktopUpdate()
	const operator = state.kind === "operator" && !installing

	return (
		<InstallAppProvider>
			<div className={`app-shell ${operator ? "app-shell-operator" : ""}`}>
				<HeaderGradient />
				<main className={`app-container ${operator ? "operator-container" : ""}`}>
					<ScreenErrorBoundary>
						{installing ? <UpdatingScreen /> : <AppScreen state={state} refresh={refresh} />}
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
