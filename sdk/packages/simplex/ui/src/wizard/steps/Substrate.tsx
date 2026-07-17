import { useState } from "react"
import { api } from "../../api"
import type { StepProps } from "../Wizard"

export function StepSubstrate({ state, setState }: StepProps) {
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()

	const generate = async () => {
		setBusy(true)
		setError(undefined)
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

	const freeDisplay = state.balanceCheck
		? (Number(state.balanceCheck.free) / 10 ** state.balanceCheck.decimals).toLocaleString()
		: null

	return (
		<div className="card">
			<h2>Hyperbridge account</h2>
			<p className="hint">
				Orders are won by submitting signed bids to Hyperbridge. This Substrate account signs those bid extrinsics
				and must hold BRIDGE tokens to pay their fees — the fees are claimed back automatically after fills.
			</p>

			<div className="row">
				<button type="button" onClick={generate} disabled={busy}>
					Generate a new account
				</button>
				<span className="hint">or paste an existing hex seed / mnemonic below</span>
			</div>

			{state.generatedMnemonic && (
				<div>
					<div className="mnemonic">{state.generatedMnemonic}</div>
					<p className="hint">
						⚠ Back this mnemonic up now — it is shown once and controls the BRIDGE funds you deposit.
					</p>
				</div>
			)}

			<label className="field">
				<span>Substrate private key (hex seed or mnemonic)</span>
				<input
					type="password"
					value={state.substrateKey}
					onChange={(e) =>
						setState((s) => ({ ...s, substrateKey: e.target.value, generatedMnemonic: undefined, balanceCheck: undefined }))
					}
					onBlur={deriveFromPasted}
				/>
			</label>

			{state.substrateAddress && (
				<p className="hint">
					Hyperbridge address: <span className="mono">{state.substrateAddress}</span> — fund this account with
					BRIDGE tokens before filling.
				</p>
			)}

			<label className="field">
				<span>Hyperbridge WebSocket URL (used to submit and track bids)</span>
				<input
					type="text"
					value={state.hyperbridgeWsUrl}
					onChange={(e) => setState((s) => ({ ...s, hyperbridgeWsUrl: e.target.value }))}
				/>
			</label>

			<div className="row">
				<button type="button" onClick={checkBalance} disabled={busy || !state.substrateKey.trim()}>
					Check BRIDGE balance
				</button>
				{state.balanceCheck &&
					(state.balanceCheck.funded ? (
						<span className="badge ok">funded — {freeDisplay} BRIDGE</span>
					) : (
						<span className="badge warn">not funded yet (you can continue and fund later)</span>
					))}
			</div>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
