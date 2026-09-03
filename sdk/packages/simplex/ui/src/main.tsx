import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import "./styles/foundations.css"
import "./styles/review.css"
import "./styles/markets.css"
import "./styles/setup-controls.css"
import "./styles/treasury.css"
import "./styles/substrate.css"
import "./styles/controls.css"
import "./styles/recovery.css"
import "./styles/operator.css"
import "./styles/responsive.css"
import "./styles/pwa.css"

if (import.meta.env.PROD && "serviceWorker" in navigator) {
	navigator.serviceWorker.register("./sw.js", { scope: "./" }).catch(() => {
		// The app remains fully usable when a browser or host disables service workers.
	})
}

createRoot(document.getElementById("root")!).render(
	<StrictMode>
		<App />
	</StrictMode>,
)
