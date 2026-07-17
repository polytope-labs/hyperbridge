import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import type { ConfigDto, RebalancingDto } from "../types"

export function Operations() {
	const [rebalancing, setRebalancing] = useState<RebalancingDto>()
	const [config, setConfig] = useState<ConfigDto>()
	const [allowlistInput, setAllowlistInput] = useState("")
	const [message, setMessage] = useState<string>()
	const [error, setError] = useState<string>()

	const load = useCallback(async () => {
		try {
			const [rebalancingDto, configDto] = await Promise.all([
				api.get<RebalancingDto>("/api/rebalancing"),
				api.get<ConfigDto>("/api/config"),
			])
			setRebalancing(rebalancingDto)
			setConfig(configDto)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])

	useEffect(() => {
		load()
	}, [load])

	const act = async (fn: () => Promise<unknown>, done: string) => {
		setMessage(undefined)
		setError(undefined)
		try {
			await fn()
			setMessage(done)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	return (
		<div>
			<div className="card">
				<h2>Rebalancing</h2>
				{!rebalancing && <p className="hint">Loading…</p>}
				{rebalancing && !rebalancing.configured && (
					<p className="hint">Not configured — set base balances in the config file to enable it.</p>
				)}
				{rebalancing?.configured && (
					<div>
						<p className="hint">
							Triggers when a chain drops to {(1 - (rebalancing.triggerPercentage ?? 0)) * 100}% of its base
							balance.
						</p>
						<pre className="toml">{JSON.stringify(rebalancing.triggers, null, 2)}</pre>
					</div>
				)}
			</div>

			<div className="card">
				<h2>Vault treasury</h2>
				<p className="hint">
					Manual controls for the ERC-4626 treasury (available when a [vault] is configured): sweep idle wallet
					balance in now, or redeem positions back to the wallet.
				</p>
				<div className="row">
					<button type="button" onClick={() => act(() => api.post("/api/vault/sweep"), "Sweep executed")}>
						Sweep now
					</button>
					<button type="button" onClick={() => act(() => api.post("/api/vault/redeem"), "Positions redeemed")}>
						Redeem all
					</button>
				</div>
			</div>

			<div className="card">
				<h2>Allowlist</h2>
				<p className="hint">
					Only fill orders placed by these addresses. Applies immediately and is saved to the config. Leave empty
					and save to accept orders from everyone.
				</p>
				<div className="row">
					<input
						type="text"
						style={{ flex: 1 }}
						placeholder="0x…, 0x… (comma or space separated)"
						value={allowlistInput}
						onChange={(e) => setAllowlistInput(e.target.value)}
					/>
					<button
						type="button"
						onClick={() =>
							act(
								() =>
									api.put("/api/allowlist", {
										users: allowlistInput
											.split(/[\s,]+/)
											.map((s) => s.trim())
											.filter(Boolean),
									}),
								"Allowlist updated",
							)
						}
					>
						Save
					</button>
				</div>
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
