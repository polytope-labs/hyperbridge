import { useEffect, useState } from "react"
import { api } from "../api"
import type { SetupDefaults } from "../types"
import { assembleConfig, chainLabels, type WizardState } from "./state"
import { formatSetupStartError } from "./startError"

export type LaunchPhase = "editing" | "saving" | "starting" | "failed"

/** How often the start is checked. The solver is usually up within seconds. */
const START_POLL_MS = 500

/**
 * Saves the wizard's config and starts the solver, then follows the start until
 * the dashboard can open.
 */
export function useLaunch(state: WizardState, defaults: SetupDefaults) {
	const [phase, setPhase] = useState<LaunchPhase>("editing")
	const [error, setError] = useState<string>()
	// Whether the last attempt got as far as writing the config. A refused save wrote nothing.
	const [saved, setSaved] = useState(false)

	useEffect(() => {
		if (phase !== "starting") return
		// One outcome is acted on. A slow check can still be in flight when the next one answers.
		let stopped = false
		const stop = () => {
			stopped = true
			clearInterval(timer)
		}
		const timer = setInterval(async () => {
			try {
				const status = await api.get<{ state: string; error?: string }>("/api/setup/start-status")
				if (stopped) return
				if (status.state === "running") {
					stop()
					window.location.href = "/"
				} else if (status.state === "failed") {
					stop()
					setPhase("failed")
					setError(status.error)
				}
			} catch {
				// server may briefly be busy while booting; keep polling
			}
		}, START_POLL_MS)
		return stop
	}, [phase])

	const start = () => {
		if (phase === "saving" || phase === "starting") return
		setPhase("saving")
		setSaved(false)
		setError(undefined)
		void api
			.post("/api/setup/save-and-start", {
				config: assembleConfig(state, defaults),
				chainLabels: chainLabels(state),
				chainIds: state.chains.map((c) => c.meta.chainId),
			})
			.then(() => {
				setSaved(true)
				setPhase("starting")
			})
			.catch((err) => {
				setPhase("failed")
				setError(err instanceof Error ? err.message : String(err))
			})
	}

	return {
		phase,
		saved,
		start,
		error: error
			? formatSetupStartError(error, new Map(state.chains.map((chain) => [chain.meta.chainId, chain.meta.label])))
			: undefined,
	}
}

/** Past this, the launch is slower than usual, and the screen says so. */
const LAUNCH_EXPECTED_SECS = 15

/**
 * The launch screen. Boot reports no stages, so the bar is indeterminate and the
 * clock is the only measure — anything that ticked stages off would be invented.
 */
export function LaunchProgress() {
	const [elapsed, setElapsed] = useState(0)

	useEffect(() => {
		const startedAt = Date.now()
		const timer = setInterval(() => setElapsed(Math.floor((Date.now() - startedAt) / 1000)), 1000)
		return () => clearInterval(timer)
	}, [])

	const clock = `${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`

	return (
		<div className="card review-launching">
			<div className="review-launching-heading">
				<span className="review-launching-spinner" aria-hidden="true" />
				<h2>Starting the solver…</h2>
				<span className="review-launching-clock">{clock}</span>
			</div>
			<div className="review-launching-bar" role="progressbar" aria-label="Starting the solver">
				<span />
			</div>
			<p className="hint">
				Resolving chains and setting up EIP-7702 delegation. This page switches to the dashboard automatically.
			</p>
			{elapsed >= LAUNCH_EXPECTED_SECS ? (
				<p className="hint">Still working. Slow RPC endpoints can stretch this.</p>
			) : null}
		</div>
	)
}
