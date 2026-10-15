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

/**
 * Child-process exits sent as events. With no minidumps, these replace native
 * crash reports for the renderer, GPU and utility processes: process type,
 * reason and exit code, but no memory.
 */
const CHILD_PROCESS_EVENTS: Array<"crashed" | "oom" | "abnormal-exit" | "launch-failed" | "integrity-failure"> = [
	"crashed",
	"oom",
	"abnormal-exit",
	"launch-failed",
	"integrity-failure",
]

/** Must run before the app is ready so startup failures are captured. Returns whether reporting is on. */
export function startDesktopErrorReporting(version: string, packaged: boolean): boolean {
	if (!DESKTOP_DSN || !errorReportingEnabled()) return false
	// bootstrap enables source maps before loading main.js; its map ships in the asar.
	Sentry.init({
		dsn: DESKTOP_DSN,
		release: `simplex-desktop@${version}`,
		environment: packaged ? "production" : "development",
		ipcMode: Sentry.IPCMode.Classic,
		sendDefaultPii: false,
		// Tracing is not used; skipping OpenTelemetry keeps the bundled main process free of loader hooks.
		skipOpenTelemetrySetup: true,
		integrations: (defaults) => [
			...defaults.filter(
				(integration) => !EXCLUDED_INTEGRATIONS.has(integration.name) && integration.name !== "ChildProcess",
			),
			Sentry.childProcessIntegration({ events: CHILD_PROCESS_EVENTS }),
		],
		beforeBreadcrumb: (breadcrumb) => scrubBreadcrumb(breadcrumb),
		beforeSend: (event) => scrubEvent(event),
	})
	return true
}

export function reportDesktopError(error: unknown): void {
	Sentry.captureException(error)
}

/** The solver died or stopped answering while it was meant to be running. */
export function reportSolverFailure(
	status: { state: string; detail?: string },
	previous: { state: string; version?: string },
): void {
	Sentry.captureMessage(`Simplex solver ${status.state === "unreachable" ? "became unreachable" : "stopped"}`, {
		level: "error",
		tags: { "solver.previous_state": previous.state, "solver.version": previous.version ?? "unknown" },
		extra: { detail: status.detail },
	})
}

/** The previous desktop session ended without a clean quit. */
export function reportUncleanExit(previous: { version?: string; startedAt?: string; platform?: string }): void {
	Sentry.captureMessage("Previous Simplex Desktop session ended without a clean quit", {
		level: "fatal",
		tags: { "previous.version": previous.version ?? "unknown" },
		extra: { previousStartedAt: previous.startedAt, previousPlatform: previous.platform },
	})
}

/** Delivers queued events before the process exits. */
export function flushDesktopErrorReports(timeoutMs = 2_000): Promise<boolean> {
	return Sentry.flush(timeoutMs)
}
