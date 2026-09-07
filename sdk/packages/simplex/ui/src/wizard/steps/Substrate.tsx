import { useState } from "react"
import { api } from "../../api"
import { Field } from "../../components/Field"
import { RecoveryPhraseDialog } from "../../components/RecoveryPhraseDialog"
import type { StepProps } from "../Wizard"

export function StepSubstrate({ state, setState }: StepProps) {
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()
	const [addressCopyStatus, setAddressCopyStatus] = useState<"idle" | "copied" | "failed">("idle")

	const generate = async () => {
		setBusy(true)
		setError(undefined)
		setAddressCopyStatus("idle")
		try {
			const { mnemonic, address } = await api.post<{ mnemonic: string; address: string }>(
				"/api/setup/generate-substrate-key",
			)
			setState((s) => ({ ...s, substrateKey: mnemonic, generatedMnemonic: mnemonic, substrateAddress: address }))
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	const deriveFromPasted = async () => {
		if (!state.substrateKey.trim()) return
		try {
			const { address } = await api.post<{ address: string }>("/api/setup/generate-substrate-key", {
				key: state.substrateKey.trim(),
			})
			setAddressCopyStatus("idle")
			setState((s) => ({ ...s, substrateAddress: address, generatedMnemonic: undefined }))
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	const checkBalance = async () => {
		setBusy(true)
		setError(undefined)
		try {
			const res = await api.post<{ funded: boolean; free: string; decimals: number; address: string }>(
				"/api/setup/check-substrate-balance",
				{ wsUrl: state.hyperbridgeWsUrl.trim(), key: state.substrateKey.trim() },
			)
			setState((s) => ({
				...s,
				substrateAddress: res.address,
				balanceCheck: { funded: res.funded, free: res.free, decimals: res.decimals },
			}))
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	const copyAddress = async () => {
		if (!state.substrateAddress) return
		try {
			await navigator.clipboard.writeText(state.substrateAddress)
			setAddressCopyStatus("copied")
		} catch {
			setAddressCopyStatus("failed")
		}
	}

	const freeDisplay = state.balanceCheck
		? (Number(state.balanceCheck.free) / 10 ** state.balanceCheck.decimals).toLocaleString()
		: null

	return (
		<div className="substrate-card">
			<div className="substrate-account-options">
				<section className="substrate-account-option" aria-labelledby="new-account-title">
					<h3 id="new-account-title">New account</h3>
					<p>Generate a dedicated account and save its recovery phrase.</p>
					<button
						type="button"
						className="primary substrate-generate-button"
						onClick={generate}
						disabled={busy}
					>
						Generate account <span aria-hidden="true">→</span>
					</button>
				</section>

				<section
					className="substrate-account-option substrate-import-option"
					aria-labelledby="existing-account-title"
				>
					<h3 id="existing-account-title">Existing account</h3>
					<p>Import the account you already use for Hyperbridge.</p>
					<Field
						label="Recovery phrase or hex seed"
						type="password"
						required
						value={state.substrateKey}
						onChange={(substrateKey) => {
							setAddressCopyStatus("idle")
							setState((s) => ({
								...s,
								substrateKey,
								generatedMnemonic: undefined,
								balanceCheck: undefined,
							}))
						}}
						onBlur={deriveFromPasted}
					/>
				</section>
			</div>

			{state.generatedMnemonic ? (
				<RecoveryPhraseDialog
					phrase={state.generatedMnemonic}
					onAcknowledge={() => setState((s) => ({ ...s, generatedMnemonic: undefined }))}
				/>
			) : null}

			{state.substrateAddress ? (
				<section className="substrate-address" aria-labelledby="substrate-address-title">
					<div>
						<span id="substrate-address-title">Hyperbridge address</span>
						<code>{state.substrateAddress}</code>
					</div>
					<button type="button" className="secondary substrate-copy-button" onClick={copyAddress}>
						<span aria-hidden="true">⧉</span>
						{addressCopyStatus === "copied" ? "Copied" : "Copy"}
					</button>
					<p className="substrate-address-status" data-state={addressCopyStatus} role="status">
						{addressCopyStatus === "copied"
							? "Address copied to clipboard."
							: addressCopyStatus === "failed"
								? "Clipboard unavailable."
								: "Fund this account with BRIDGE before taking orders."}
					</p>
				</section>
			) : null}

			<section className="substrate-connection" aria-labelledby="substrate-connection-title">
				<div className="substrate-section-heading">
					<h3 id="substrate-connection-title">Connection</h3>
					<p>Used to submit bids and track execution.</p>
				</div>
				<Field
					label="Hyperbridge WebSocket URL"
					required
					value={state.hyperbridgeWsUrl}
					onChange={(hyperbridgeWsUrl) => setState((s) => ({ ...s, hyperbridgeWsUrl }))}
				/>
			</section>

			<section className="substrate-balance" aria-labelledby="substrate-balance-title">
				<div className="substrate-section-heading">
					<h3 id="substrate-balance-title">BRIDGE balance</h3>
					<p>Check whether this account is ready to submit bids.</p>
				</div>
				<button
					type="button"
					className="secondary substrate-balance-button"
					onClick={checkBalance}
					disabled={busy || !state.substrateKey.trim()}
				>
					Check balance
				</button>
				{state.balanceCheck ? (
					<div
						className="substrate-balance-state"
						data-state={state.balanceCheck.funded ? "funded" : "unfunded"}
					>
						<span aria-hidden="true" />
						<div>
							<strong>
								{state.balanceCheck.funded ? `${freeDisplay} BRIDGE available` : "Not funded yet"}
							</strong>
							<p>
								{state.balanceCheck.funded
									? "This account is ready to submit bids."
									: "You can continue setup, then fund this address before taking orders."}
							</p>
						</div>
					</div>
				) : null}
			</section>
			{error ? <p className="error">{error}</p> : null}
		</div>
	)
}
