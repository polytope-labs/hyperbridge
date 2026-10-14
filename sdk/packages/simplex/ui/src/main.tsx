import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import { reactRootErrorHandler, startErrorReporting } from "./lib/error-reporting"
import "./styles/hyperbridge-ui.css"
import "./styles/foundations.css"
import "./styles/review.css"
import "./styles/markets.css"
import "./styles/setup-controls.css"
import "./styles/treasury.css"
import "./styles/substrate.css"
import "./styles/controls.css"
import "./styles/recovery.css"
import "./styles/operator.css"
import "./styles/logs.css"
import "./styles/responsive.css"
import "./styles/pwa.css"
import "./styles/drawer.css"
import "./styles/desktop-access.css"

startErrorReporting()

if (import.meta.env.PROD && "serviceWorker" in navigator) {
	navigator.serviceWorker.register("./sw.js", { scope: "./" }).catch(() => {
		// The app remains fully usable when a browser or host disables service workers.
	})
}

createRoot(document.getElementById("root")!, {
	onUncaughtError: reactRootErrorHandler(),
	onCaughtError: reactRootErrorHandler(),
	onRecoverableError: reactRootErrorHandler(),
}).render(
	<StrictMode>
		<App />
	</StrictMode>,
)
