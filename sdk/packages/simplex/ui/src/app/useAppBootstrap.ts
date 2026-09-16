import { useCallback, useEffect, useRef, useState } from "react"
import { api } from "../api"
import type { SetupDefaults, Status, StatusOperator } from "../types"

export type AppBootstrapState =
	| { kind: "connecting" }
	| { kind: "error"; message: string }
	| { kind: "loading-setup" }
	| { kind: "version-skew"; desktopVersion: string; solverVersion: string }
	| { kind: "operator"; status: StatusOperator }
	| { kind: "setup"; defaults: SetupDefaults }

export function resolveBootstrapState(
	status: Status | undefined,
	defaults: SetupDefaults | undefined,
	error: string | undefined,
	desktopVersion: string | undefined,
): AppBootstrapState {
	if (error) return { kind: "error", message: error }
	if (!status) return { kind: "connecting" }
	const solverVersion = typeof status.version === "string" ? status.version : "unknown"
	if (desktopVersion && desktopVersion !== solverVersion) {
		return { kind: "version-skew", desktopVersion, solverVersion }
	}
	if (status.mode === "operator") {
		return { kind: "operator", status }
	}
	if (!defaults) return { kind: "loading-setup" }
	return { kind: "setup", defaults }
}

/** Owns the application's boot lifecycle so App only composes the shell. */
export function useAppBootstrap() {
	const [status, setStatus] = useState<Status>()
	const [defaults, setDefaults] = useState<SetupDefaults>()
	const [error, setError] = useState<string>()
	const [desktopVersion, setDesktopVersion] = useState<string>()
	const requestId = useRef(0)

	const refresh = useCallback(async () => {
		const currentRequest = ++requestId.current
		try {
			const result = await api.getWithMeta<Status>("/api/status")
			const next = result.body
			if (currentRequest !== requestId.current) return
			setDesktopVersion(result.response.headers.get("x-simplex-desktop-version") ?? undefined)
			setStatus(next)
			setError(undefined)
			if (next.mode === "operator") {
				setDefaults(undefined)
				return
			}

			const nextDefaults = await api.get<SetupDefaults>("/api/setup/defaults")
			if (currentRequest === requestId.current) setDefaults(nextDefaults)
		} catch (cause) {
			if (currentRequest === requestId.current) {
				setError(cause instanceof Error ? cause.message : String(cause))
			}
		}
	}, [])

	useEffect(() => {
		void refresh()
		return () => {
			requestId.current += 1
		}
	}, [refresh])

	return { state: resolveBootstrapState(status, defaults, error, desktopVersion), refresh }
}
