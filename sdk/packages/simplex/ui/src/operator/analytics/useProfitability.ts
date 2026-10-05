import { useEffect, useState } from "react"
import { api, ApiError } from "../../api"
import type { ProfitabilityDto, ProfitPeriod } from "../../types"

/** How often a summary refreshes itself: fills land without the operator acting. */
const POLL_MS = 60_000

/**
 * What the operator's buys and sells earned over `period`, kept current while the page is open.
 *
 * Buckets are cut on the viewer's clock, so the request says how far that clock is from UTC. A
 * summary for one period is never shown under another: changing period clears it until the new
 * one arrives.
 */
export function useProfitability(period: ProfitPeriod) {
	const [summary, setSummary] = useState<ProfitabilityDto>()
	const [error, setError] = useState<string>()

	useEffect(() => {
		// A slow answer for a period the operator has since left must not replace the current one.
		let live = true
		const load = async () => {
			try {
				const tz = new Date().getTimezoneOffset()
				const next = await api.get<ProfitabilityDto>(`/api/analytics/profitability?period=${period}&tz=${tz}`)
				if (!live) return
				setSummary(next)
				setError(undefined)
			} catch (err) {
				if (!live) return
				setError(err instanceof ApiError ? err.message : "Could not read profitability")
			}
		}
		void load()
		const timer = setInterval(() => void load(), POLL_MS)
		return () => {
			live = false
			clearInterval(timer)
		}
	}, [period])

	const current = summary?.period === period ? summary : undefined
	return { summary: current, loading: current === undefined && error === undefined, error }
}
