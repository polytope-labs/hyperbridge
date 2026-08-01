import { useCallback, useEffect, useState } from "react"

/** Wraps a mutating API call with the shared message/error surface. */
export function useAction() {
	const [message, setMessage] = useState<string>()
	const [error, setError] = useState<string>()
	const run = useCallback(async (fn: () => Promise<unknown>, done?: string) => {
		setMessage(undefined)
		setError(undefined)
		try {
			await fn()
			if (done) setMessage(done)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])
	return { run, message, error }
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
