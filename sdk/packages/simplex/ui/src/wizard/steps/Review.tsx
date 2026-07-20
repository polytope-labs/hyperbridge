import { useEffect, useState } from "react"
import { api } from "../../api"
import { assembleConfig, chainLabels } from "../state"
import type { StepProps } from "../Wizard"

type Phase = "review" | "starting" | "failed"

export function StepReview({ state, defaults }: StepProps) {
	const [toml, setToml] = useState<string>()
	const [previewError, setPreviewError] = useState<string>()
	const [phase, setPhase] = useState<Phase>("review")
	const [startError, setStartError] = useState<string>()

	useEffect(() => {
		let cancelled = false
		const config = assembleConfig(state, defaults)
		api.post<{ ok: boolean; toml?: string; error?: string }>("/api/setup/preview", {
			config,
			chainLabels: chainLabels(state),
		})
			.then((res) => {
				if (cancelled) return
				setToml(res.toml)
				setPreviewError(undefined)
			})
			.catch((err) => {
				if (cancelled) return
				setToml(undefined)
				setPreviewError(err instanceof Error ? err.message : String(err))
			})
		return () => {
			cancelled = true
		}
	}, [state, defaults])

	useEffect(() => {
		if (phase !== "starting") return
		const timer = setInterval(async () => {
			try {
				const status = await api.get<{ state: string; error?: string }>("/api/setup/start-status")
				if (status.state === "running") {
					window.location.href = "/"
				} else if (status.state === "failed") {
					setPhase("failed")
					setStartError(status.error)
				}
			} catch {
				// server may briefly be busy while booting; keep polling
			}
		}, 2000)
		return () => clearInterval(timer)
	}, [phase])

	const saveAndStart = async () => {
		setStartError(undefined)
		try {
			await api.post("/api/setup/save-and-start", {
				config: assembleConfig(state, defaults),
				chainLabels: chainLabels(state),
			})
			setPhase("starting")
		} catch (err) {
			setPhase("failed")
			setStartError(err instanceof Error ? err.message : String(err))
		}
	}

	if (phase === "starting") {
		return (
			<div className="card">
				<h2>Starting the filler…</h2>
				<p className="hint">
					Resolving chains, hydrating funding venues and setting up EIP-7702 delegation — this takes up to a
					minute. This page switches to the dashboard automatically.
				</p>
			</div>
		)
	}

	return (
		<div>
			<div className="card">
				<h2>Before the filler can fill</h2>
				<p className="hint">
					· Fund the filler wallet with stablecoins (USDC/USDT) on every chain — gas is covered by the paymaster,
					paid in USDC.
					<br />· Fund the Hyperbridge account with BRIDGE tokens for bid fees (claimed back automatically).
				</p>
			</div>

			<div className="card">
				<h2>Config preview (secrets masked)</h2>
				<p className="hint">
					Written to {defaults.configPath} with permissions 600. The file on disk contains the real secrets — keep
					it private and out of version control.
				</p>
				{previewError && <p className="error">{previewError}</p>}
				{toml && <pre className="toml">{toml}</pre>}
			</div>

			{phase === "failed" && (
				<div className="card">
					<p className="error">The filler failed to start: {startError}</p>
					<p className="hint">
						The config file was written — fix the problem (funding, endpoints) and try again, or edit the file and
						run `simplex run` manually.
					</p>
				</div>
			)}

			<button type="button" className="primary" disabled={!toml} onClick={saveAndStart}>
				Save & start the filler
			</button>
		</div>
	)
}
