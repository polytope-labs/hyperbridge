import * as Sentry from "@sentry/electron/main"
import {
	ERROR_REPORTING_ENV,
	errorReportingDisabled,
	scrubBreadcrumb,
	scrubEvent,
	sentryIngestOrigin,
} from "@hyperbridge/simplex/error-reporting"

// Baked in by tsup.config.ts from the release workflow. Empty in local builds.
declare const __SIMPLEX_DESKTOP_SENTRY_DSN__: string | undefined
declare const __SIMPLEX_UI_SENTRY_DSN__: string | undefined

const DESKTOP_DSN = typeof __SIMPLEX_DESKTOP_SENTRY_DSN__ === "string" ? __SIMPLEX_DESKTOP_SENTRY_DSN__ : ""
const UI_DSN = typeof __SIMPLEX_UI_SENTRY_DSN__ === "string" ? __SIMPLEX_UI_SENTRY_DSN__ : ""

/**
 * Integrations that could ship key material or wallet contents off the machine:
 * native minidumps hold process memory, LocalVariables* capture frame values,
 * Screenshots captures the window. PreloadInjection is unneeded because the
 * renderer reports through the shared UI's own browser SDK.
 */
const EXCLUDED_INTEGRATIONS = new Set([
	"SentryMinidump",
	"ElectronMinidump",
	"LocalVariables",
	"LocalVariablesAsync",
	"LocalVariablesSync",
	"Screenshots",
	"PreloadInjection",
	"RendererEventLoopBlock",
])

export function errorReportingEnabled(env: NodeJS.ProcessEnv = process.env): boolean {
	return !errorReportingDisabled(env[ERROR_REPORTING_ENV])
}

/** The Sentry origin the renderer CSP admits, or undefined when the UI must not report. */
export function rendererReportingOrigin(env: NodeJS.ProcessEnv = process.env): string | undefined {
	return UI_DSN && errorReportingEnabled(env) ? sentryIngestOrigin(UI_DSN) : undefined
}

/** Must run before the app is ready so startup failures are captured. */
export function startDesktopErrorReporting(version: string, packaged: boolean): void {
	if (!DESKTOP_DSN || !errorReportingEnabled()) return
	// dist/main.js.map ships in the asar, so stacks arrive already mapped to src/.
	process.setSourceMapsEnabled(true)
	Sentry.init({
		dsn: DESKTOP_DSN,
		release: `simplex-desktop@${version}`,
		environment: packaged ? "production" : "development",
		ipcMode: Sentry.IPCMode.Classic,
		sendDefaultPii: false,
		// Tracing is not used; skipping OpenTelemetry keeps the bundled main process free of loader hooks.
		skipOpenTelemetrySetup: true,
		integrations: (defaults) => defaults.filter((integration) => !EXCLUDED_INTEGRATIONS.has(integration.name)),
		beforeBreadcrumb: (breadcrumb) => scrubBreadcrumb(breadcrumb),
		beforeSend: (event) => scrubEvent(event),
	})
}

export function reportDesktopError(error: unknown): void {
	Sentry.captureException(error)
}

/** Delivers queued events before the process exits. */
export function flushDesktopErrorReports(timeoutMs = 2_000): Promise<boolean> {
	return Sentry.flush(timeoutMs)
}
