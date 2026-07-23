import { useCallback, useState } from "react"
import { api } from "../api"
import { AddressListEditor } from "../components/AddressListEditor"
import { useAction, usePolling } from "../lib/hooks"
import type { ConfigDto, RebalancingDto } from "../types"

interface VaultRow {
	chain: string
	vault: string
	threshold: string
	minBalance: string
	redeemOnShutdown: boolean
}

interface BalanceRow {
	symbol: "USDC" | "USDT"
	chainId: string
	amount: string
}

export function Operations(props: { chains: number[] }) {
	const [rebalancing, setRebalancing] = useState<RebalancingDto>()
	const [config, setConfig] = useState<ConfigDto>()
	const [allowlist, setAllowlist] = useState<string[]>([])
	const [vaultRows, setVaultRows] = useState<VaultRow[]>()
	const [trigger, setTrigger] = useState("0.5")
	const [balanceRows, setBalanceRows] = useState<BalanceRow[]>()
	const { run: act, message, error } = useAction()

	const load = useCallback(async () => {
		const [rebalancingDto, configDto] = await Promise.all([
			api.get<RebalancingDto>("/api/rebalancing"),
			api.get<ConfigDto>("/api/config"),
		])
		setRebalancing(rebalancingDto)
		setConfig(configDto)
		setAllowlist(configDto.allowlistUsers)
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
		// seed the editors once from the running config; later edits are local until saved
		setTrigger((current) =>
			current === "0.5" && rebalancingDto.triggerPercentage !== undefined
				? String(rebalancingDto.triggerPercentage)
				: current,
		)
		setBalanceRows(
			(current) =>
				current ??
				(["USDC", "USDT"] as const).flatMap((symbol) =>
					Object.entries(rebalancingDto.baseBalances?.[symbol] ?? {}).map(([chainId, amount]) => ({
						symbol,
						chainId,
						amount,
					})),
				),
		)
	}, [])
	usePolling(useCallback(() => act(load), [act, load]))

	const saveRebalancing = () =>
		act(async () => {
			const baseBalances: { USDC?: Record<string, string>; USDT?: Record<string, string> } = {}
			for (const row of balanceRows ?? []) {
				if (!row.amount.trim()) continue
				baseBalances[row.symbol] = { ...(baseBalances[row.symbol] ?? {}), [row.chainId]: row.amount.trim() }
			}
			const res = await api.put<{ applied: boolean; restartNeeded: boolean }>("/api/rebalancing", {
				triggerPercentage: Number(trigger),
				baseBalances,
			})
			await load()
			if (res.restartNeeded) throw new Error("Saved to config — restart the filler to start the rebalancing loop")
		}, "Rebalancing updated")

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
				<h2>Rebalancing</h2>
				{!rebalancing && <p className="hint">Loading…</p>}
				{rebalancing && (
					<div>
						<p className="hint">
							Tops up a chain's stablecoin balance from richer chains when it drops below the trigger fraction
							of its base. Changes apply immediately{!rebalancing.configured && " after a restart"} and are
							saved to the config.
						</p>
						<div className="row">
							<label className="field" style={{ maxWidth: "12rem", margin: 0 }}>
								<span>Trigger fraction (0–1)</span>
								<input type="text" value={trigger} onChange={(e) => setTrigger(e.target.value)} />
							</label>
						</div>
						{(balanceRows ?? []).map((row, index) => (
							// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
							<div className="row" key={index} style={{ margin: "0.4rem 0" }}>
								<select
									value={row.symbol}
									onChange={(e) =>
										setBalanceRows((rows) =>
											(rows ?? []).map((r, i) => (i === index ? { ...r, symbol: e.target.value as "USDC" | "USDT" } : r)),
										)
									}
								>
									<option>USDC</option>
									<option>USDT</option>
								</select>
								<select
									value={row.chainId}
									onChange={(e) =>
										setBalanceRows((rows) => (rows ?? []).map((r, i) => (i === index ? { ...r, chainId: e.target.value } : r)))
									}
								>
									{props.chains.map((id) => (
										<option key={id} value={String(id)}>
											chain {id}
										</option>
									))}
								</select>
								<input
									type="text"
									placeholder="base balance (USD)"
									style={{ maxWidth: "11rem" }}
									value={row.amount}
									onChange={(e) =>
										setBalanceRows((rows) => (rows ?? []).map((r, i) => (i === index ? { ...r, amount: e.target.value } : r)))
									}
								/>
								<button type="button" onClick={() => setBalanceRows((rows) => (rows ?? []).filter((_, i) => i !== index))}>
									✕
								</button>
							</div>
						))}
						<div className="row">
							<button
								type="button"
								onClick={() =>
									setBalanceRows((rows) => [
										...(rows ?? []),
										{ symbol: "USDC", chainId: String(props.chains[0] ?? ""), amount: "10000" },
									])
								}
							>
								+ Add base balance
							</button>
							<button
								type="button"
								className="primary"
								disabled={(balanceRows ?? []).filter((r) => r.amount.trim()).length === 0}
								onClick={saveRebalancing}
							>
								Save rebalancing
							</button>
						</div>
						{rebalancing.configured && rebalancing.triggers !== undefined && (
							<pre className="toml" style={{ marginTop: "0.7rem" }}>
								{JSON.stringify(rebalancing.triggers, null, 2)}
							</pre>
						)}
					</div>
				)}
			</div>

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
									chain {id}
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

			<div className="card">
				<div className="spread">
					<h2>Log level</h2>
					{config && (
						<select
							value={config.logLevel}
							onChange={(e) =>
								act(async () => {
									await api.put("/api/log-level", { level: e.target.value })
									await load()
								}, `Log level set to ${e.target.value}`)
							}
						>
							{["trace", "debug", "info", "warn", "error"].map((level) => (
								<option key={level}>{level}</option>
							))}
						</select>
					)}
				</div>
				<p className="hint">Applies immediately and is saved to the config.</p>
			</div>

			<div className="card">
				<h2>Active config (secrets masked)</h2>
				{config && (
					<div>
						<p className="hint">
							{config.configPath} — chains, endpoints and signer changes require an edit + restart.
						</p>
						<pre className="toml">{config.toml}</pre>
					</div>
				)}
			</div>

			{message && <p className="hint">✓ {message}</p>}
			{error && <p className="error">{error}</p>}
		</div>
	)
}
