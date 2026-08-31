import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import type { ActivityEventDto, BidDto, BidStatsDto } from "../types"

const TYPE_BADGE: Record<ActivityEventDto["type"], string> = {
	detected: "",
	filled: "ok",
	executed: "",
	skipped: "warn",
	rebalance: "",
}

function describe(event: ActivityEventDto): string {
	switch (event.type) {
		case "detected":
			return "order detected"
		case "filled":
			return `filled${event.volumeUsd ? ` $${event.volumeUsd.toLocaleString()}` : ""}${
				event.profitUsd ? ` (+$${event.profitUsd.toFixed(2)})` : ""
			}`
		case "executed":
			return event.success ? `executed via ${event.strategy ?? "?"}` : `execution failed: ${event.reason ?? "?"}`
		case "skipped":
			return `skipped — ${event.reason ?? "?"}`
		case "rebalance":
			return `rebalance — ${event.reason ?? (event.success ? "ok" : "failed")}`
	}
}

// SSE frames can land while the initial fetch is in flight; merge by id so
// neither source overwrites the other and cap long-running sessions.
function mergeEvents(current: ActivityEventDto[], incoming: ActivityEventDto[]): ActivityEventDto[] {
	const byId = new Map<number, ActivityEventDto>()
	for (const event of [...current, ...incoming]) byId.set(event.id, event)
	return [...byId.values()].sort((a, b) => b.id - a.id).slice(0, 1000)
}

export function Activity() {
	const [events, setEvents] = useState<ActivityEventDto[]>([])
	const [bids, setBids] = useState<BidDto[]>([])
	const [stats, setStats] = useState<BidStatsDto | null>(null)
	const [live, setLive] = useState(false)
	const [error, setError] = useState<string>()

	const load = useCallback(async () => {
		try {
			const [orderFeed, bidFeed] = await Promise.all([
				api.get<{ events: ActivityEventDto[] }>("/api/activity/orders?limit=100"),
				api.get<{ bids: BidDto[]; stats: BidStatsDto | null }>("/api/activity/bids?limit=50"),
			])
			setEvents((current) => mergeEvents(current, orderFeed.events))
			setBids(bidFeed.bids)
			setStats(bidFeed.stats)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])

	useEffect(() => {
		load()
	}, [load])

	// Live tail: new activity rows arrive over SSE and are prepended.
	useEffect(() => {
		const source = new EventSource("/api/events")
		source.onopen = () => setLive(true)
		source.onerror = () => setLive(false)
		source.onmessage = (message) => {
			const event = JSON.parse(message.data) as ActivityEventDto
			setEvents((current) => mergeEvents(current, [event]))
		}
		return () => source.close()
	}, [])

	const loadOlder = async () => {
		const oldest = events[events.length - 1]
		if (!oldest) return
		const older = await api.get<{ events: ActivityEventDto[] }>(
			`/api/activity/orders?limit=100&before=${oldest.id}`,
		)
		setEvents((current) => mergeEvents(current, older.events))
	}

	return (
		<div className="operator-page-content">
			{stats ? (
				<section className="operator-metrics" aria-label="Bid summary">
					<div>
						<span>Total bids</span>
						<strong>{stats.total}</strong>
					</div>
					<div>
						<span>Successful</span>
						<strong>{stats.successful}</strong>
					</div>
					<div>
						<span>Failed</span>
						<strong>{stats.failed}</strong>
					</div>
					<div>
						<span>Retracted</span>
						<strong>{stats.retracted}</strong>
					</div>
				</section>
			) : null}
			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Orders</span>
						<h2>Order activity</h2>
					</div>
					<span className={`badge ${live ? "ok" : "warn"}`}>{live ? "live" : "reconnecting…"}</span>
				</div>
				{events.length === 0 && (
					<p className="operator-empty">No activity yet. Events appear as orders are detected.</p>
				)}
				{events.length > 0 && (
					<table>
						<thead>
							<tr>
								<th>Time</th>
								<th>Order</th>
								<th>Event</th>
							</tr>
						</thead>
						<tbody>
							{events.map((event) => (
								<tr key={event.id}>
									<td>{new Date(event.ts).toLocaleTimeString()}</td>
									<td>{event.orderId ? <CopyHash value={event.orderId} /> : "—"}</td>
									<td>
										<span className={`badge ${TYPE_BADGE[event.type]}`}>{describe(event)}</span>
									</td>
								</tr>
							))}
						</tbody>
					</table>
				)}
				{events.length >= 100 && (
					<button type="button" style={{ marginTop: "0.6rem" }} onClick={loadOlder}>
						Load older
					</button>
				)}
			</section>

			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Hyperbridge</span>
						<h2>Submitted bids</h2>
					</div>
					{stats?.pendingRetraction ? (
						<span className="badge warn">{stats.pendingRetraction} pending</span>
					) : null}
				</div>
				{bids.length === 0 && <p className="operator-empty">No bids recorded.</p>}
				{bids.length > 0 && (
					<table>
						<thead>
							<tr>
								<th>Created</th>
								<th>Commitment</th>
								<th>Status</th>
							</tr>
						</thead>
						<tbody>
							{bids.map((bid) => (
								<tr key={bid.id}>
									<td>{bid.createdAt}</td>
									<td>
										<CopyHash value={bid.commitment} chars={14} />
									</td>
									<td>
										{bid.retracted ? (
											<span className="badge">retracted</span>
										) : bid.success ? (
											<span className="badge ok">successful</span>
										) : (
											<span className="badge err">{bid.error ?? "failed"}</span>
										)}
									</td>
								</tr>
							))}
						</tbody>
					</table>
				)}
			</section>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
