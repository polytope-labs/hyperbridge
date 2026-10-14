import * as Sentry from "@sentry/react"
import { scrubBreadcrumb, scrubEvent } from "@/config/error-reporting"
import { isNativeDesktopProtocol } from "./runtime"

// Baked in by ui/vite.config.ts. Empty in local and community builds, which never report.
declare const __SIMPLEX_UI_SENTRY_DSN__: string | undefined
declare const __SIMPLEX_UI_RELEASE__: string | undefined

const DSN = typeof __SIMPLEX_UI_SENTRY_DSN__ === "string" ? __SIMPLEX_UI_SENTRY_DSN__ : ""
const RELEASE = typeof __SIMPLEX_UI_RELEASE__ === "string" ? __SIMPLEX_UI_RELEASE__ : undefined

/**
 * Asks the solver whether its operator opted out with SIMPLEX_ERROR_REPORTING=off.
 * A locked desktop answers 423; the desktop shell enforces its opt-out through
 * the renderer CSP, so that case reports. Any other failure stays silent.
 */
async function solverAllowsReporting(): Promise<boolean> {
	try {
		const response = await fetch("/health", { cache: "no-store" })
		if (response.status === 423) return true
		if (!response.ok) return false
		const health = (await response.json()) as { errorReporting?: boolean }
		return health.errorReporting !== false
	} catch {
		return false
	}
}

export function startErrorReporting(): void {
	if (!DSN) return
	const allowed = solverAllowsReporting()
	Sentry.init({
		dsn: DSN,
		release: RELEASE,
		environment: isNativeDesktopProtocol(window.location.protocol) ? "desktop" : "web",
		sendDefaultPii: false,
		beforeBreadcrumb: (breadcrumb) => scrubBreadcrumb(breadcrumb),
		beforeSend: async (event) => ((await allowed) ? scrubEvent(event) : null),
	})
}

/** React 19 root hooks; onCaughtError covers errors ScreenErrorBoundary renders. */
export const reactRootErrorHandler = Sentry.reactErrorHandler
