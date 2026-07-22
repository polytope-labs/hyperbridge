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

export function Activity() {
	const [events, setEvents] = useState<ActivityEventDto[]>([])
	const [bids, setBids] = useState<BidDto[]>([])
	const [stats, setStats] = useState<BidStatsDto | null>(null)
	const [live, setLive] = useState(false)
	const [error, setError] = useState<string>()

	// SSE frames can land while the initial fetch is in flight; every state
	// update merges by id so neither source overwrites the other.
	const mergeEvents = (current: ActivityEventDto[], incoming: ActivityEventDto[]): ActivityEventDto[] => {
		const byId = new Map<number, ActivityEventDto>()
		for (const event of [...current, ...incoming]) byId.set(event.id, event)
		// cap high enough that "Load older" pages are never cut, low enough to bound very long sessions
		return [...byId.values()]
			.sort((a, b) => b.id - a.id)
			.slice(0, 1000)
	}

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
		<div>
			<div className="card">
				<div className="spread">
					<h2>Order activity</h2>
					<span className={`badge ${live ? "ok" : "warn"}`}>{live ? "live" : "reconnecting…"}</span>
				</div>
				{events.length === 0 && <p className="hint">No activity yet — events appear as orders are detected.</p>}
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
			</div>

			<div className="card">
				<h2>Bids</h2>
				{stats && (
					<p className="hint">
						{stats.total} total · {stats.successful} successful · {stats.failed} failed · {stats.retracted}{" "}
						retracted · {stats.pendingRetraction} pending retraction
					</p>
				)}
				{bids.length === 0 && <p className="hint">No bids recorded.</p>}
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
			</div>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
