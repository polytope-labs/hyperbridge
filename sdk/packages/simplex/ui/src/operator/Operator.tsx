import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import { CurveEditor, type EditorPoint } from "../components/CurveEditor"
import type { AdminStrategyDto, BalanceSnapshot, PricePoint, StatusOperator } from "../types"

function formatUptime(seconds: number): string {
	const h = Math.floor(seconds / 3600)
	const m = Math.floor((seconds % 3600) / 60)
	return h > 0 ? `${h}h ${m}m` : `${m}m ${seconds % 60}s`
}

export function Operator(props: { status: StatusOperator; refresh: () => void }) {
	const { status, refresh } = props
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

			<div className="card">
				<div className="spread">
					<h2>Fill control</h2>
					<button type="button" className="primary" onClick={togglePause}>
						{status.paused ? "Resume filling" : "Pause filling"}
					</button>
				</div>
				<p className="hint">
					Pause keeps monitoring orders but stops analysing and filling new ones; in-flight fills complete. Orders
					arriving while paused are dropped, not queued. The paused state survives restarts.
				</p>
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

			<div className="card">
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

			<div className="card">
				<h2>Price curves</h2>
				<p className="hint">
					Edits apply to the running strategies immediately and are lost on restart; the config file is re-read on
					boot.
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
			await api.put(`/api/strategies/${strategy.index}/curves`, {
				...(strategy.bid ? { bidPriceCurve: toPricePoints(bid) } : {}),
				...(strategy.ask ? { askPriceCurve: toPricePoints(ask) } : {}),
			})
			setMessage("Applied — in memory only, lost on restart")
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
