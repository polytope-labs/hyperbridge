import { useCallback, useState } from "react"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { CurveEditor, fromPricePoints, toPricePoints, type EditorPoint } from "../components/CurveEditor"
import { PillTabs } from "../components/PillTabs"
import { useAction, usePolling } from "../lib/hooks"
import type { AdminStrategyDto, BalanceSnapshot, StatusOperator } from "../types"
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
	const [loadError, setLoadError] = useState<string>()
	const [stopped, setStopped] = useState(false)
	const { run, error } = useAction()

	const load = useCallback(async () => {
		try {
			// Status is polled too so runtime changes (overfill self-halt, an
			// external pause) surface without a manual action.
			const [balanceSnapshot, strategyList] = await Promise.all([
				api.get<BalanceSnapshot>("/api/balances"),
				api.get<{ strategies: AdminStrategyDto[] }>("/api/strategies"),
				refresh(),
			])
			setBalances(balanceSnapshot)
			setStrategies(strategyList.strategies)
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
					<StrategyCurves key={strategy.index} strategy={strategy} onApplied={load} />
				))}
			</div>
			{(error ?? loadError) && <p className="error">{error ?? loadError}</p>}
		</div>
	)
}


function StrategyCurves(props: { strategy: AdminStrategyDto; onApplied: () => void }) {
	const { strategy, onApplied } = props
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
					the ask everywhere — the gap is your spread.
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
				{message && <span className="badge ok">{message}</span>}
				{error && <span className="badge err">{error}</span>}
			</div>
		</div>
	)
}
