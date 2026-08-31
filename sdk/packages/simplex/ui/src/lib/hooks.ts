import { useCallback, useEffect, useRef, useState } from "react"

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
