import { useEffect, useState } from "react"
import { api } from "../../api"
import { CopyHash } from "../../components/CopyHash"
import { CheckIcon } from "../../components/InterfaceIcons"
import { assembleConfig, chainLabels, enabledChains } from "../state"
import type { StepProps } from "../Wizard"

type Phase = "review" | "saving" | "starting" | "failed"

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
			chainIds: enabledChains(state).map((c) => c.meta.chainId),
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

	const saveAndStart = () => {
		if (phase === "saving") return
		setPhase("saving")
		setStartError(undefined)
		void api
			.post("/api/setup/save-and-start", {
				config: assembleConfig(state, defaults),
				chainLabels: chainLabels(state),
				chainIds: enabledChains(state).map((c) => c.meta.chainId),
			})
			.then(() => setPhase("starting"))
			.catch((err) => {
				setPhase("failed")
				setStartError(err instanceof Error ? err.message : String(err))
			})
	}

	if (phase === "starting") {
		return (
			<div className="wizard-sections review-step">
				<div className="card">
					<h2>Starting the filler…</h2>
					<p className="hint">
						Resolving chains, hydrating funding venues and setting up EIP-7702 delegation — this takes up to
						a minute. This page switches to the dashboard automatically.
					</p>
				</div>
			</div>
		)
	}

	const evmAddress =
		state.signerType === "privateKey"
			? state.signerAddress
			: state.signerType === "mpcVault"
				? state.mpcVault.accountAddress
				: state.turnkey.signWith
	const enabled = enabledChains(state)
	const needsNativeBnb = enabled.some((chain) => /bnb|bsc/i.test(chain.meta.label))
	const pathSeparator = defaults.configPath.lastIndexOf("/")
	const configDirectory = pathSeparator >= 0 ? defaults.configPath.slice(0, pathSeparator + 1) : ""
	const configFilename = pathSeparator >= 0 ? defaults.configPath.slice(pathSeparator + 1) : defaults.configPath
	const directoryParts = configDirectory.split("/").filter(Boolean)
	const compactConfigDirectory =
		directoryParts.length > 3 ? `…/${directoryParts.slice(-3).join("/")}/` : configDirectory

	return (
		<div className="wizard-sections review-step">
			<section className="card review-accounts-section">
				<header className="review-section-heading">
					<span className="markets-kicker">Funding checklist</span>
					<h2>Fund your accounts</h2>
					<p>These are the two identities Simplex will use after launch.</p>
				</header>

				<div className="review-account-list">
					{evmAddress && (
						<article className="review-account-row">
							<div className="review-account-heading">
								<div>
									<h3>Filler wallet</h3>
									<span>EVM execution account</span>
								</div>
								<span className="review-account-network">EVM</span>
							</div>
							<p>
								Fund with USDC or USDT on every enabled chain.
								{needsNativeBnb
									? " Keep native BNB available for gas on BNB Chain."
									: " Gas is covered by the paymaster."}
							</p>
							<div className="review-account-value">
								<CopyHash value={evmAddress} chars={42} copyLabel="Copy filler wallet address" />
							</div>
						</article>
					)}
					{state.substrateAddress && (
						<article className="review-account-row">
							<div className="review-account-heading">
								<div>
									<h3>Hyperbridge account</h3>
									<span>Bid submission account</span>
								</div>
								<span className="review-account-network">SUBSTRATE</span>
							</div>
							<p>Fund with BRIDGE for bid fees. Claimed fees return automatically after fills.</p>
							<div className="review-account-value">
								<CopyHash
									value={state.substrateAddress}
									chars={64}
									copyLabel="Copy Hyperbridge account address"
								/>
							</div>
						</article>
					)}
				</div>
			</section>

			<section className="card review-config-section">
				<header className="review-section-heading">
					<span className="markets-kicker">Local output</span>
					<h2>Configuration file</h2>
					<p>Simplex writes a private, editable TOML file to this location.</p>
				</header>

				<div className="review-config-path">
					<span className="review-config-path-label">Destination</span>
					<CopyHash
						value={defaults.configPath}
						chars={defaults.configPath.length}
						copyLabel="Copy config path"
					>
						<span className="review-path-directory">{compactConfigDirectory}</span>
						<strong>{configFilename}</strong>
					</CopyHash>
				</div>
				<p className="review-config-security">
					<span aria-hidden="true">◆</span>
					Only your user account can read or edit this file. Keep it private and out of version control.
				</p>

				{previewError ? (
					<div className="review-validation" data-state="error" role="alert">
						<span className="review-validation-icon" aria-hidden="true">
							!
						</span>
						<div>
							<strong>Configuration needs attention</strong>
							<p>{previewError}</p>
						</div>
					</div>
				) : toml ? (
					<div className="review-validation" data-state="ready" role="status">
						<span className="review-validation-icon" aria-hidden="true">
							<CheckIcon />
						</span>
						<div>
							<strong>Ready to launch</strong>
							<p>The generated configuration passed validation.</p>
						</div>
					</div>
				) : (
					<div className="review-validation" data-state="checking" role="status">
						<span className="review-validation-icon review-validation-spinner" aria-hidden="true" />
						<div>
							<strong>Checking configuration</strong>
							<p>Validating the generated TOML before launch.</p>
						</div>
					</div>
				)}
			</section>

			{phase === "failed" && (
				<div className="card">
					<p className="error">The filler failed to start: {startError}</p>
					<p className="hint">
						The config file was written — fix the problem (funding, endpoints) and try again, or edit the
						file and run `simplex run` manually.
					</p>
				</div>
			)}

			<div className="review-launch-row">
				<button
					type="button"
					className="primary review-start-button"
					disabled={!toml || phase === "saving"}
					onClick={saveAndStart}
				>
					{phase === "saving" ? "Saving configuration…" : "Save config & start Simplex"}
					{phase !== "saving" && <span aria-hidden="true">→</span>}
				</button>
				<p>The dashboard opens automatically when the solver is ready.</p>
			</div>
		</div>
	)
}
