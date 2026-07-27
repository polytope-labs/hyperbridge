import { useCallback, useState } from "react"
import { api } from "../api"
import { AddressListEditor } from "../components/AddressListEditor"
import { useAction, usePolling } from "../lib/hooks"
import type { ConfigDto } from "../types"

interface VaultRow {
	chain: string
	vault: string
	threshold: string
	minBalance: string
	redeemOnShutdown: boolean
}

export function Operations(props: { chains: number[]; chainLabels?: Record<string, string> }) {
	const [config, setConfig] = useState<ConfigDto>()
	const [allowlist, setAllowlist] = useState<string[]>([])
	const [vaultRows, setVaultRows] = useState<VaultRow[]>()
	const { run: act, message, error } = useAction()

	const chainLabel = (id: number | string) => props.chainLabels?.[String(id)] ?? `chain ${id}`

	const load = useCallback(async () => {
		const configDto = await api.get<ConfigDto>("/api/config")
		setConfig(configDto)
		setAllowlist(configDto.allowlistUsers)
		// seed the editor once from the running config; later edits are local until saved
		setVaultRows(
			(current) =>
				current ??
				configDto.vaults.map((v) => ({
					chain: v.chain,
					vault: v.vault,
					threshold: v.threshold ?? "",
					minBalance: v.minBalance ?? "",
					redeemOnShutdown: v.redeemOnShutdown ?? false,
				})),
		)
	}, [])
	usePolling(useCallback(() => act(load), [act, load]))

	const saveVaults = () =>
		act(async () => {
			const rows = (vaultRows ?? []).filter((r) => r.vault.trim())
			const res = await api.put<{ applied: boolean; restartNeeded: boolean }>("/api/vault", {
				vaults: rows.map((r) => ({
					chain: r.chain,
					vault: r.vault.trim(),
					...(r.threshold.trim() ? { threshold: r.threshold.trim() } : {}),
					...(r.minBalance.trim() ? { minBalance: r.minBalance.trim() } : {}),
					redeemOnShutdown: r.redeemOnShutdown,
				})),
			})
			await load()
			if (res.restartNeeded) throw new Error("Saved to config — restart the filler to activate the vault treasury")
		}, "Vault treasury updated")

	return (
		<div>
			<div className="card">
				<h2>Vault treasury</h2>
				<p className="hint">
					ERC-4626 vaults per chain (one per asset). Threshold and min balance are USD-denominated. Edits
					re-hydrate the running venue{config && !config.vaultConfigured && " after a restart"} and are saved to
					the config.
				</p>
				{(vaultRows ?? []).map((row, index) => (
					// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
					<div className="row" key={index} style={{ margin: "0.4rem 0" }}>
						<select
							value={row.chain}
							onChange={(e) =>
								setVaultRows((rows) => (rows ?? []).map((r, i) => (i === index ? { ...r, chain: e.target.value } : r)))
							}
						>
							{props.chains.map((id) => (
								<option key={id} value={`EVM-${id}`}>
									{chainLabel(id)}
								</option>
							))}
						</select>
						<input
							type="text"
							placeholder="vault address 0x…"
							style={{ flex: 1 }}
							value={row.vault}
							onChange={(e) =>
								setVaultRows((rows) => (rows ?? []).map((r, i) => (i === index ? { ...r, vault: e.target.value } : r)))
							}
						/>
						<input
							type="text"
							placeholder="sweep threshold (USD)"
							style={{ maxWidth: "10rem" }}
							value={row.threshold}
							onChange={(e) =>
								setVaultRows((rows) => (rows ?? []).map((r, i) => (i === index ? { ...r, threshold: e.target.value } : r)))
							}
						/>
						<input
							type="text"
							placeholder="min balance (USD)"
							style={{ maxWidth: "10rem" }}
							value={row.minBalance}
							onChange={(e) =>
								setVaultRows((rows) => (rows ?? []).map((r, i) => (i === index ? { ...r, minBalance: e.target.value } : r)))
							}
						/>
						<label className="row" style={{ whiteSpace: "nowrap" }} title="Redeem this position to the wallet on graceful shutdown">
							<input
								type="checkbox"
								checked={row.redeemOnShutdown}
								onChange={(e) =>
									setVaultRows((rows) =>
										(rows ?? []).map((r, i) => (i === index ? { ...r, redeemOnShutdown: e.target.checked } : r)),
									)
								}
							/>
							redeem on shutdown
						</label>
						<button type="button" onClick={() => setVaultRows((rows) => (rows ?? []).filter((_, i) => i !== index))}>
							✕
						</button>
					</div>
				))}
				<div className="row">
					<button
						type="button"
						onClick={() =>
							setVaultRows((rows) => [
								...(rows ?? []),
								{ chain: `EVM-${props.chains[0] ?? ""}`, vault: "", threshold: "", minBalance: "", redeemOnShutdown: false },
							])
						}
					>
						+ Add vault
					</button>
					<button type="button" className="primary" disabled={vaultRows === undefined} onClick={saveVaults}>
						Save vaults
					</button>
					{config?.vaultConfigured && (
						<span style={{ marginLeft: "auto" }} className="row">
							<button type="button" onClick={() => act(() => api.post("/api/vault/sweep"), "Sweep executed")}>
								Sweep now
							</button>
							<button type="button" onClick={() => act(() => api.post("/api/vault/redeem"), "Positions redeemed")}>
								Redeem all
							</button>
						</span>
					)}
				</div>
			</div>

			<div className="card">
				<h2>Allowlist</h2>
				<p className="hint">
					Only fill orders placed by these addresses. Changes apply immediately and are saved to the config. An
					empty list accepts orders from everyone.
				</p>
				<AddressListEditor
					addresses={allowlist}
					onChange={(users) =>
						act(async () => {
							await api.put("/api/allowlist", { users })
							setAllowlist(users)
						}, "Allowlist updated")
					}
				/>
			</div>

			{config && (
				<p className="hint">
					Config: <span className="mono">{config.configPath}</span> — chains, endpoints and signer changes require
					an edit + restart.
				</p>
			)}

			{message && <p className="hint">✓ {message}</p>}
			{error && <p className="error">{error}</p>}
		</div>
	)
}
