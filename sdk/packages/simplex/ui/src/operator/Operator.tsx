import { useCallback, useState } from "react"
import { bookCrossedAt } from "@/config/interpolated-curve"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { CurveEditor, fromPricePoints, toPricePoints, type EditorPoint } from "../components/CurveEditor"
import { PillTabs } from "../components/PillTabs"
import { useAction, usePolling } from "../lib/hooks"
import type { AdminStrategyDto, BalanceSnapshot, ConfigDto, StatusOperator } from "../types"
import { Activity } from "./Activity"
import { Operations } from "./Operations"
import { Wallet } from "./Wallet"

type Tab = "overview" | "activity" | "wallet" | "operations"

const PAGE_TABS = [
	{ value: "overview", label: "overview" },
	{ value: "activity", label: "activity" },
	{ value: "wallet", label: "wallet" },
	{ value: "operations", label: "operations" },
] as const

function formatUptime(seconds: number): string {
	const h = Math.floor(seconds / 3600)
	const m = Math.floor((seconds % 3600) / 60)
	return h > 0 ? `${h}h ${m}m` : `${m}m ${seconds % 60}s`
}

export function Operator(props: { status: StatusOperator; refresh: () => void }) {
	const { status, refresh } = props
	const [tab, setTab] = useState<Tab>("overview")
	const [balances, setBalances] = useState<BalanceSnapshot>()
	const [strategies, setStrategies] = useState<AdminStrategyDto[]>([])
	const [config, setConfig] = useState<ConfigDto>()
	const [showAddMarket, setShowAddMarket] = useState(false)
	const [loadError, setLoadError] = useState<string>()
	const [stopped, setStopped] = useState(false)
	const { run, error } = useAction()

	const load = useCallback(async () => {
		try {
			// Status is polled too so runtime changes (overfill self-halt, an
			// external pause) surface without a manual action.
			const [balanceSnapshot, strategyList, configDto] = await Promise.all([
				api.get<BalanceSnapshot>("/api/balances"),
				api.get<{ strategies: AdminStrategyDto[] }>("/api/strategies"),
				api.get<ConfigDto>("/api/config"),
				refresh(),
			])
			setBalances(balanceSnapshot)
			setStrategies(strategyList.strategies)
			setConfig(configDto)
			setLoadError(undefined)
		} catch (err) {
			setLoadError(err instanceof Error ? err.message : String(err))
		}
	}, [refresh])
	usePolling(load, 30_000)

	const togglePause = () =>
		run(async () => {
			await api.post(status.paused ? "/api/resume" : "/api/pause")
			refresh()
		})

	const resetHalt = () =>
		run(async () => {
			await api.post("/api/reset-halt")
			refresh()
		})

	const stopFiller = () => {
		if (!window.confirm("Stop the filler? In-flight fills drain, vault positions may unwind, and the process exits.")) {
			return
		}
		return run(async () => {
			await api.post("/api/stop")
			setStopped(true)
		})
	}

	if (stopped) {
		return (
			<div className="card">
				<h2>Filler stopping</h2>
				<p className="hint">
					In-flight fills are draining and the process will exit. Restart it with `simplex run` — a persisted
					pause state is honored on the next boot.
				</p>
			</div>
		)
	}

	return (
		<div>
			<div className="spread">
				<h1>Simplex operator</h1>
				<div className="row">
					{status.paused ? <span className="badge warn">paused</span> : <span className="badge ok">running</span>}
					<span className="badge">up {formatUptime(status.uptimeSec)}</span>
					<span className="badge">v{status.version}</span>
				</div>
			</div>
			<p className="hint">
				Config: <span className="mono">{status.configPath}</span> · chains {status.chains.join(", ")} · markets{" "}
				{status.strategyTypes.join(", ")}
			</p>
			{status.addresses && (
				<div className="card" style={{ padding: "0.7rem 1rem" }}>
					<div className="row" style={{ flexWrap: "wrap", columnGap: "0.6rem" }}>
						<span className="hint" style={{ minWidth: "8rem" }}>
							Filler wallet
						</span>
						<span className="mono" style={{ fontSize: "1rem" }}>
							<CopyHash value={status.addresses.evm} chars={42} />
						</span>
					</div>
					{status.addresses.substrate && (
						<div className="row" style={{ flexWrap: "wrap", columnGap: "0.6rem" }}>
							<span className="hint" style={{ minWidth: "8rem" }}>
								Hyperbridge
							</span>
							<span className="mono" style={{ fontSize: "1rem" }}>
								<CopyHash value={status.addresses.substrate} chars={64} />
							</span>
						</div>
					)}
				</div>
			)}

			<PillTabs options={PAGE_TABS} value={tab} onChange={setTab} />

			{tab === "activity" && <Activity />}
			{tab === "wallet" && <Wallet chainLabels={status.chainLabels} />}
			{tab === "operations" && <Operations chains={status.chains} chainLabels={status.chainLabels} />}

			<div className="card" style={tab !== "overview" ? { display: "none" } : undefined}>
				<div className="spread">
					<h2>Fill control</h2>
					<div className="row">
						<button type="button" className="primary" onClick={togglePause}>
							{status.paused ? "Resume filling" : "Pause filling"}
						</button>
						<button type="button" onClick={stopFiller}>
							Stop filler
						</button>
					</div>
				</div>
				<p className="hint">
					Pause keeps monitoring orders but stops analysing and filling new ones; in-flight fills complete. Orders
					arriving while paused are dropped, not queued. The paused state survives restarts. Stop drains in-flight
					fills and exits the process.
				</p>
				{status.halted.length > 0 && (
					<div>
						<p className="error">
							Overfill protection halted strategy {status.halted.map((i) => `#${i}`).join(", ")}: consecutive
							venue-priced clamps suggest a stale or manipulated price source. Investigate the venue before
							resuming.
						</p>
						<button type="button" onClick={resetHalt}>
							Reset halt & resume
						</button>
					</div>
				)}
				{Object.entries(status.watchOnly).some(([, v]) => v) && (
					<p className="hint">
						Watch-only chains:{" "}
						{Object.entries(status.watchOnly)
							.filter(([, v]) => v)
							.map(([id]) => id)
							.join(", ")}
					</p>
				)}
			</div>

			<div className="card" style={tab !== "overview" ? { display: "none" } : undefined}>
				<h2>Balances</h2>
				{!balances?.updatedAt && <p className="hint">First refresh lands within a minute of startup…</p>}
				{balances && balances.chains.length > 0 && (
					<table>
						<thead>
							<tr>
								<th>Chain</th>
								<th>Native</th>
								<th>USDC</th>
								<th>USDT</th>
								<th>Exotics</th>
							</tr>
						</thead>
						<tbody>
							{balances.chains.map((row) => (
								<tr key={row.chainId}>
									<td>{row.chainId}</td>
									<td>{row.native ? `${row.native.amount.toFixed(4)} ${row.native.symbol}` : "—"}</td>
									<td>{row.usdc?.toLocaleString() ?? "—"}</td>
									<td>{row.usdt?.toLocaleString() ?? "—"}</td>
									<td>
										{row.exotics?.length
											? row.exotics.map((e) => `${e.amount.toLocaleString()} ${e.symbol}`).join(", ")
											: "—"}
									</td>
								</tr>
							))}
						</tbody>
					</table>
				)}
				{balances?.hyperbridge && (
					<p className="hint">
						BRIDGE on Hyperbridge: {balances.hyperbridge.free.toLocaleString()} free /{" "}
						{balances.hyperbridge.reserved.toLocaleString()} reserved — bids stop when this runs dry.
					</p>
				)}
			</div>

			<div className="card" style={tab !== "overview" ? { display: "none" } : undefined}>
				<h2>Price curves</h2>
				<p className="hint">
					Edits apply to the running markets immediately and are persisted to the config file (which is
					regenerated with standard comments).
				</p>
				{strategies.length === 0 && <p className="hint">No curve-priced markets configured.</p>}
				{strategies.map((strategy) => (
					<StrategyCurves
						key={strategy.index}
						strategy={strategy}
						onApplied={load}
						removable={strategies.length > 1}
						hasVaults={(config?.vaults.length ?? 0) > 0}
					/>
				))}
				{showAddMarket ? (
					<AddMarketForm
						symbols={marketSymbols(config)}
						chains={status.chains}
						chainLabel={(id) => status.chainLabels?.[String(id)] ?? `chain ${id}`}
						onAdded={() => {
							setShowAddMarket(false)
							void load()
						}}
						onCancel={() => setShowAddMarket(false)}
					/>
				) : (
					<button type="button" onClick={() => setShowAddMarket(true)}>
						+ Add market
					</button>
				)}
			</div>
			{(error ?? loadError) && <p className="error">{error ?? loadError}</p>}
		</div>
	)
}


/** Registry + [assets] symbols on the running chains, from the send-token catalog. */
function marketSymbols(config: ConfigDto | undefined): string[] {
	const symbols = new Set<string>()
	for (const tokens of Object.values(config?.sendTokens ?? {})) {
		for (const token of tokens) {
			if (token.symbol !== "native") symbols.add(token.symbol)
		}
	}
	return [...symbols].sort()
}

function StrategyCurves(props: {
	strategy: AdminStrategyDto
	onApplied: () => void
	removable: boolean
	hasVaults: boolean
}) {
	const { strategy, onApplied, removable, hasVaults } = props
	const [bid, setBid] = useState<EditorPoint[]>(() => fromPricePoints(strategy.bid))
	const [ask, setAsk] = useState<EditorPoint[]>(() => fromPricePoints(strategy.ask))
	// One-sided LP: an absent side of a curve-priced cross-asset market can be
	// opened by submitting a curve for it. Same-token markets stay ask-only.
	const [enableBid, setEnableBid] = useState(false)
	const [enableAsk, setEnableAsk] = useState(false)
	const [message, setMessage] = useState<string>()
	const [error, setError] = useState<string>()

	const token0 = strategy.token0 || "token0"
	const token1 = strategy.token1 || "token1"
	const crossedAt =
		!strategy.sameToken && !strategy.referenceOnly && (strategy.bid || enableBid) && (strategy.ask || enableAsk)
			? (bookCrossedAt(toPricePoints(bid), toPricePoints(ask))?.amount ?? null)
			: null
	const title = strategy.referenceOnly
		? `Market #${strategy.index} · ${token0}/${token1} — reference price feed`
		: strategy.sameToken
			? `Market #${strategy.index} · ${strategy.exotic} — same-asset transfers`
			: `Market #${strategy.index} ${strategy.exotic ? `· ${strategy.exotic}` : ""}`

	if (strategy.pricingMode === "venue") {
		return (
			<div>
				<h2 style={{ fontSize: "0.95rem" }}>{title}</h2>
				<p className="hint">Prices derive from on-chain venues (Uniswap V4) and cannot be edited here.</p>
			</div>
		)
	}

	const apply = async () => {
		setMessage(undefined)
		setError(undefined)
		try {
			const res = await api.put<{ persisted: boolean }>(`/api/strategies/${strategy.index}/curves`, {
				...(strategy.bid || enableBid ? { bidPriceCurve: toPricePoints(bid) } : {}),
				...(strategy.ask || enableAsk ? { askPriceCurve: toPricePoints(ask) } : {}),
			})
			setMessage(res.persisted ? "Applied & saved to config" : "Applied in memory — config file could not be written")
			setEnableBid(false)
			setEnableAsk(false)
			onApplied()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	const remove = async () => {
		const vaultNote = hasVaults
			? " Funds already swept into vaults stay there and remain redeemable from the operations tab."
			: ""
		if (
			!window.confirm(
				`Remove market ${strategy.exotic ?? `${token0}/${token1}`}? The filler stops bidding on it immediately and it is deleted from the config.${vaultNote}`,
			)
		) {
			return
		}
		setMessage(undefined)
		setError(undefined)
		try {
			await api.del(`/api/strategies/${strategy.index}`)
			onApplied()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	return (
		<div style={{ marginBottom: "1rem" }}>
			<h2 style={{ fontSize: "0.95rem" }}>{title}</h2>
			{strategy.sameToken && (
				<p className="hint">
					Ask-only: the price is the fraction of the input paid back out. Keep every point strictly below 1 —
					the gap to 1 is the spread on each fill.
				</p>
			)}
			{strategy.referenceOnly && (
				<p className="hint">
					Price feed only: edits update the USD anchor rate for confirmation sizing. This market never fills
					orders.
				</p>
			)}
			{!strategy.sameToken && !strategy.referenceOnly && (
				<p className="hint">
					Prices are {token1} per {token0}: the bid is the {token1} you receive per {token0} paid out when
					buying, the ask is the {token1} you pay out per {token0} received when selling. Keep the bid above
					the ask everywhere — the gap is your spread. Delete every point on a side and Apply to close that
					direction (one-sided LP).
				</p>
			)}
			{crossedAt !== null && (
				<p className="hint">
					⚠ The book is crossed at order size {crossedAt} (bid at or below ask) — both sides still fill at
					their own curve, but a full round trip at these prices loses money. Leave it only if deliberate.
				</p>
			)}
			<div className="row" style={{ alignItems: "flex-start", gap: "2rem" }}>
				{(strategy.bid || enableBid) && (
					<div>
						<p className="hint">Bid — filler buys {token1}{!strategy.bid && " (enabling this side)"}</p>
						<CurveEditor
							points={bid}
							onChange={setBid}
							amountLabel={`Order size (${token0})`}
							valueLabel={`${token1} received per ${token0}`}
							minPoints={strategy.sameToken || strategy.referenceOnly ? 1 : 0}
						/>
					</div>
				)}
				{(strategy.ask || enableAsk) && (
					<div>
						<p className="hint">
							{strategy.sameToken
								? "Ask — fraction paid out (below 1)"
								: `Ask — filler sells ${token1}${!strategy.ask ? " (enabling this side)" : ""}`}
						</p>
						<CurveEditor
							points={ask}
							onChange={setAsk}
							amountLabel={`Order size (${token0})`}
							valueLabel={strategy.sameToken ? "Price (below 1)" : `${token1} paid per ${token0}`}
							minPoints={strategy.sameToken || strategy.referenceOnly ? 1 : 0}
						/>
					</div>
				)}
			</div>
			{!strategy.sameToken && !strategy.referenceOnly && !strategy.bid && !enableBid && (
				<button
					type="button"
					onClick={() => {
						setBid([{ amount: "0", value: "" }])
						setEnableBid(true)
					}}
				>
					Enable bid side (one-sided LP → both directions)
				</button>
			)}
			{!strategy.sameToken && !strategy.referenceOnly && !strategy.ask && !enableAsk && (
				<button
					type="button"
					style={{ marginLeft: "0.5rem" }}
					onClick={() => {
						setAsk([{ amount: "0", value: "" }])
						setEnableAsk(true)
					}}
				>
					Enable ask side (one-sided LP → both directions)
				</button>
			)}
			<div className="row" style={{ marginTop: "0.5rem" }}>
				<button type="button" className="primary" onClick={apply}>
					Apply
				</button>
				{removable && (
					<button type="button" onClick={remove}>
						Remove market
					</button>
				)}
				{message && <span className="badge ok">{message}</span>}
				{error && <span className="badge err">{error}</span>}
			</div>
		</div>
	)
}

const CUSTOM = "__custom__"

/**
 * Inline new-market form: registry symbols by default, one custom token with
 * per-chain addresses when a side picks "custom tokenâ¦". Same-token pairs
 * collapse to the ask-only editor the engine requires.
 */
function AddMarketForm(props: {
	symbols: string[]
	chains: number[]
	chainLabel: (id: number | string) => string
	onAdded: () => void
	onCancel: () => void
}) {
	const { symbols, chains, chainLabel, onAdded, onCancel } = props
	const [token0, setToken0] = useState("USDC")
	const [token1, setToken1] = useState(() => symbols.find((s) => s !== "USDC") ?? CUSTOM)
	const [customSymbol, setCustomSymbol] = useState("")
	const [customAddresses, setCustomAddresses] = useState<Record<string, string>>({})
	const [verified, setVerified] = useState<Record<string, string>>({})
	const [maxOrderSize, setMaxOrderSize] = useState("5000")
	const [bidEnabled, setBidEnabled] = useState(true)
	const [askEnabled, setAskEnabled] = useState(true)
	const [bid, setBid] = useState<EditorPoint[]>([{ amount: "0", value: "" }])
	const [ask, setAsk] = useState<EditorPoint[]>([{ amount: "0", value: "" }])
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()

	const customSide = token0 === CUSTOM || token1 === CUSTOM
	const resolved0 = token0 === CUSTOM ? customSymbol.trim().toUpperCase() : token0
	const resolved1 = token1 === CUSTOM ? customSymbol.trim().toUpperCase() : token1
	const sameToken = resolved0 !== "" && resolved0 === resolved1
	const crossedAt =
		!sameToken && bidEnabled && askEnabled
			? (bookCrossedAt(toPricePoints(bid), toPricePoints(ask))?.amount ?? null)
			: null

	const verifyToken = async (chainKey: string) => {
		const address = customAddresses[chainKey]?.trim()
		if (!address) return
		try {
			const res = await api.post<{ ok: boolean; symbol?: string; decimals?: number; error?: string }>(
				"/api/setup/validate-token",
				{ chain: chainKey, address },
			)
			setVerified((v) => ({
				...v,
				[chainKey]: res.ok ? `✓ ${res.symbol} (${res.decimals} decimals)` : `✗ ${res.error}`,
			}))
		} catch (err) {
			setVerified((v) => ({ ...v, [chainKey]: `✗ ${err instanceof Error ? err.message : String(err)}` }))
		}
	}

	const submit = async () => {
		setError(undefined)
		if (token0 === CUSTOM && token1 === CUSTOM) {
			setError("Add one custom token at a time")
			return
		}
		if (customSide && !customSymbol.trim()) {
			setError("The custom token needs a symbol")
			return
		}
		const addresses = Object.fromEntries(
			Object.entries(customAddresses)
				.map(([chain, address]) => [chain, address.trim()])
				.filter(([, address]) => address),
		)
		if (customSide && Object.keys(addresses).length === 0) {
			setError("The custom token needs an address on at least one chain")
			return
		}
		setBusy(true)
		try {
			const res = await api.post<{ applied: boolean; restartNeeded: boolean }>("/api/strategies", {
				token0: resolved0,
				token1: resolved1,
				maxOrderSize,
				...(!sameToken && bidEnabled ? { bidPriceCurve: toPricePoints(bid) } : {}),
				...(sameToken || askEnabled ? { askPriceCurve: toPricePoints(ask) } : {}),
				...(customSide ? { assets: { [customSymbol.trim().toUpperCase()]: addresses } } : {}),
			})
			if (res.restartNeeded) {
				setError("Saved to config — restart the filler to open the market")
			} else {
				onAdded()
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	const symbolSelect = (value: string, onChange: (v: string) => void, label: string) => (
		<label className="field" style={{ margin: 0 }}>
			<span>{label}</span>
			<select value={value} onChange={(e) => onChange(e.target.value)}>
				{symbols.map((s) => (
					<option key={s} value={s}>
						{s}
					</option>
				))}
				<option value={CUSTOM}>custom token…</option>
			</select>
		</label>
	)

	return (
		<div style={{ marginTop: "0.8rem" }}>
			<h2 style={{ fontSize: "0.95rem" }}>New market</h2>
			<div className="row" style={{ alignItems: "flex-end", flexWrap: "wrap" }}>
				{symbolSelect(token0, setToken0, "token0 (quote side)")}
				{symbolSelect(token1, setToken1, "token1")}
				<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
					<span>Max order size ({resolved0 || "token0"})</span>
					<input type="text" value={maxOrderSize} onChange={(e) => setMaxOrderSize(e.target.value)} />
				</label>
			</div>
			{customSide && (
				<div style={{ margin: "0.5rem 0" }}>
					<label className="field" style={{ maxWidth: "12rem" }}>
						<span>Custom token symbol</span>
						<input
							type="text"
							placeholder="e.g. BRZ"
							value={customSymbol}
							onChange={(e) => setCustomSymbol(e.target.value)}
						/>
					</label>
					{chains.map((id) => {
						const chainKey = `EVM-${id}`
						return (
							<div className="row" key={id} style={{ margin: "0.3rem 0" }}>
								<span style={{ minWidth: "8rem" }}>{chainLabel(id)}</span>
								<input
									type="text"
									placeholder="0x… (leave blank if not deployed here)"
									style={{ flex: 1 }}
									value={customAddresses[chainKey] ?? ""}
									onChange={(e) =>
										setCustomAddresses((a) => ({ ...a, [chainKey]: e.target.value }))
									}
								/>
								<button type="button" onClick={() => verifyToken(chainKey)}>
									Verify
								</button>
								{verified[chainKey] && <span className="hint">{verified[chainKey]}</span>}
							</div>
						)
					})}
				</div>
			)}
			{sameToken ? (
				<div>
					<p className="hint">
						Same-asset transfer market: ask-only, every price strictly below 1 — the gap to 1 is the
						spread on each fill.
					</p>
					<CurveEditor
						points={ask}
						onChange={setAsk}
						amountLabel={`Order size (${resolved0})`}
						valueLabel="Price (below 1)"
					/>
				</div>
			) : (
				<div className="row" style={{ alignItems: "flex-start", gap: "2rem" }}>
					<div>
						<label className="row">
							<input type="checkbox" checked={bidEnabled} onChange={(e) => setBidEnabled(e.target.checked)} />
							Bid — filler buys {resolved1 || "token1"}
						</label>
						{bidEnabled && (
							<CurveEditor
								points={bid}
								onChange={setBid}
								amountLabel={`Order size (${resolved0 || "token0"})`}
								valueLabel={`${resolved1 || "token1"} received per ${resolved0 || "token0"}`}
							/>
						)}
					</div>
					<div>
						<label className="row">
							<input type="checkbox" checked={askEnabled} onChange={(e) => setAskEnabled(e.target.checked)} />
							Ask — filler sells {resolved1 || "token1"}
						</label>
						{askEnabled && (
							<CurveEditor
								points={ask}
								onChange={setAsk}
								amountLabel={`Order size (${resolved0 || "token0"})`}
								valueLabel={`${resolved1 || "token1"} paid per ${resolved0 || "token0"}`}
							/>
						)}
					</div>
				</div>
			)}
			{crossedAt !== null && (
				<p className="hint">
					⚠ The book is crossed at order size {crossedAt} (bid at or below ask) — both sides still
					fill at their own curve, but a full round trip at these prices loses money. Leave it only if
					deliberate.
				</p>
			)}
			<div className="row" style={{ marginTop: "0.5rem" }}>
				<button type="button" className="primary" onClick={submit} disabled={busy}>
					{busy ? "Adding…" : "Add market"}
				</button>
				<button type="button" onClick={onCancel}>
					Cancel
				</button>
				{error && <span className="badge err">{error}</span>}
			</div>
		</div>
	)
}
