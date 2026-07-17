import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import { CurveEditor, type EditorPoint } from "../components/CurveEditor"
import type { AdminStrategyDto, BalanceSnapshot, PricePoint, StatusOperator } from "../types"
import { Activity } from "./Activity"
import { Operations } from "./Operations"

type Tab = "overview" | "activity" | "operations"

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
	const [error, setError] = useState<string>()

	const load = useCallback(async () => {
		try {
			const [balanceSnapshot, strategyList] = await Promise.all([
				api.get<BalanceSnapshot>("/api/balances"),
				api.get<{ strategies: AdminStrategyDto[] }>("/api/strategies"),
			])
			setBalances(balanceSnapshot)
			setStrategies(strategyList.strategies)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])

	useEffect(() => {
		load()
		const timer = setInterval(load, 30_000)
		return () => clearInterval(timer)
	}, [load])

	const togglePause = async () => {
		setError(undefined)
		try {
			await api.post(status.paused ? "/api/resume" : "/api/pause")
			refresh()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	const resetHalt = async () => {
		setError(undefined)
		try {
			await api.post("/api/reset-halt")
			refresh()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	const [stopped, setStopped] = useState(false)
	const stopFiller = async () => {
		if (!window.confirm("Stop the filler? In-flight fills drain, vault positions may unwind, and the process exits.")) {
			return
		}
		try {
			await api.post("/api/stop")
			setStopped(true)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
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
				Config: <span className="mono">{status.configPath}</span> · chains {status.chains.join(", ")} · strategies{" "}
				{status.strategyTypes.join(", ")}
			</p>

			<div className="steps">
				{(["overview", "activity", "operations"] as const).map((t) => (
					<button
						key={t}
						type="button"
						className={`step ${tab === t ? "active" : ""}`}
						style={{ cursor: "pointer" }}
						onClick={() => setTab(t)}
					>
						{t}
					</button>
				))}
			</div>

			{tab === "activity" && <Activity />}
			{tab === "operations" && <Operations />}

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
								<th>Exotic</th>
							</tr>
						</thead>
						<tbody>
							{balances.chains.map((row) => (
								<tr key={row.chainId}>
									<td>{row.chainId}</td>
									<td>{row.native ? `${row.native.amount.toFixed(4)} ${row.native.symbol}` : "—"}</td>
									<td>{row.usdc?.toLocaleString() ?? "—"}</td>
									<td>{row.usdt?.toLocaleString() ?? "—"}</td>
									<td>{row.exotic ? `${row.exotic.amount.toLocaleString()} ${row.exotic.symbol}` : "—"}</td>
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
					Edits apply to the running strategies immediately and are persisted to the config file (which is
					regenerated with standard comments).
				</p>
				{strategies.length === 0 && <p className="hint">No FX strategies configured.</p>}
				{strategies.map((strategy) => (
					<StrategyCurves key={strategy.index} strategy={strategy} onApplied={load} />
				))}
			</div>
			{error && <p className="error">{error}</p>}
		</div>
	)
}

function toEditor(points: PricePoint[] | undefined): EditorPoint[] {
	return (points ?? []).map((p) => ({ amount: p.amount, value: p.price }))
}

function toPricePoints(points: EditorPoint[]): PricePoint[] {
	return points.filter((p) => p.amount.trim() && p.value.trim()).map((p) => ({ amount: p.amount, price: p.value }))
}

function StrategyCurves(props: { strategy: AdminStrategyDto; onApplied: () => void }) {
	const { strategy, onApplied } = props
	const [bid, setBid] = useState<EditorPoint[]>(() => toEditor(strategy.bid))
	const [ask, setAsk] = useState<EditorPoint[]>(() => toEditor(strategy.ask))
	const [message, setMessage] = useState<string>()
	const [error, setError] = useState<string>()

	if (strategy.pricingMode === "venue") {
		return (
			<div>
				<h2 style={{ fontSize: "0.95rem" }}>
					Strategy #{strategy.index} {strategy.exotic && `· ${strategy.exotic}`}
				</h2>
				<p className="hint">Prices derive from on-chain venues (Uniswap V4) and cannot be edited here.</p>
			</div>
		)
	}

	const apply = async () => {
		setMessage(undefined)
		setError(undefined)
		try {
			const res = await api.put<{ persisted: boolean }>(`/api/strategies/${strategy.index}/curves`, {
				...(strategy.bid ? { bidPriceCurve: toPricePoints(bid) } : {}),
				...(strategy.ask ? { askPriceCurve: toPricePoints(ask) } : {}),
			})
			setMessage(res.persisted ? "Applied & saved to config" : "Applied in memory — config file could not be written")
			onApplied()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	return (
		<div style={{ marginBottom: "1rem" }}>
			<h2 style={{ fontSize: "0.95rem" }}>
				Strategy #{strategy.index} {strategy.exotic && `· ${strategy.exotic}`}
			</h2>
			<div className="row" style={{ alignItems: "flex-start", gap: "2rem" }}>
				{strategy.bid && (
					<div>
						<p className="hint">Bid — filler buys exotic</p>
						<CurveEditor points={bid} onChange={setBid} amountLabel="USD" valueLabel="Exotic/USD" />
					</div>
				)}
				{strategy.ask && (
					<div>
						<p className="hint">Ask — filler sells exotic</p>
						<CurveEditor points={ask} onChange={setAsk} amountLabel="USD" valueLabel="Exotic/USD" />
					</div>
				)}
			</div>
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
