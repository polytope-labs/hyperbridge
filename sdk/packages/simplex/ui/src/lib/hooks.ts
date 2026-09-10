import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react"

/** Wraps a mutating API call with the shared message/error surface. */
export function useAction() {
	const [message, setMessage] = useState<string>()
	const [error, setError] = useState<string>()
	const [pendingKeys, setPendingKeys] = useState<Set<string>>(() => new Set())
	const pendingKeysRef = useRef(new Set<string>())
	const run = useCallback(async (fn: () => Promise<unknown>, done?: string, key = "default") => {
		// State alone is not a guard: two clicks can arrive before React renders.
		// Keys keep background polling independent from operator mutations.
		if (pendingKeysRef.current.has(key)) return
		pendingKeysRef.current.add(key)
		setPendingKeys(new Set(pendingKeysRef.current))
		setMessage(undefined)
		setError(undefined)
		try {
			await fn()
			if (done) setMessage(done)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			pendingKeysRef.current.delete(key)
			setPendingKeys(new Set(pendingKeysRef.current))
		}
	}, [])
	return {
		run,
		pending: pendingKeys.size > 0,
		isPending: (key = "default") => pendingKeys.has(key),
		message,
		error,
	}
}

/** Runs `load` on mount and, when an interval is given, on a timer. */
export function usePolling(load: () => Promise<void> | void, intervalMs?: number) {
	useEffect(() => {
		void load()
		if (!intervalMs) return
		const timer = setInterval(() => void load(), intervalMs)
		return () => clearInterval(timer)
	}, [load, intervalMs])
}

/**
 * The width at which the dashboard stops being a sidebar-and-canvas layout: the
 * nav becomes a bottom bar and dialogs become drawers. Shared so "mobile" means
 * one thing across the app.
 */
export const MOBILE_QUERY = "(max-width: 600px)"

/**
 * A handheld, as opposed to a narrow window. Width alone is not enough: a phone
 * in landscape is 700-950px wide and would read as a desktop, while a desktop
 * browser dragged narrow would read as a phone. The second clause catches the
 * rotated phone by pairing a touch primary input with a short viewport, which
 * a tablet in landscape (768px+ tall) and a laptop both fail.
 */
export const HANDHELD_QUERY = "(max-width: 600px), (pointer: coarse) and (max-height: 600px)"

function useMediaQuery(query: string): boolean {
	const subscribe = useCallback(
		(notify: () => void) => {
			const media = window.matchMedia(query)
			media.addEventListener("change", notify)
			return () => media.removeEventListener("change", notify)
		},
		[query],
	)
	const getSnapshot = useCallback(() => window.matchMedia(query).matches, [query])
	// Server snapshot: there is no SSR here, but useSyncExternalStore requires one.
	return useSyncExternalStore(subscribe, getSnapshot, () => false)
}

/** Drives the layout: bottom nav bar instead of a sidebar, drawers instead of dialogs. */
export function useIsMobile(): boolean {
	return useMediaQuery(MOBILE_QUERY)
}

/** Drives what is worth showing at all — see {@link HANDHELD_QUERY}. */
export function useIsHandheld(): boolean {
	return useMediaQuery(HANDHELD_QUERY)
}
