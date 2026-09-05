import { useCallback, useEffect, useRef, useState } from "react"
import { INIT_CHAINS } from "@/cli/init/chains"
import { parseChainKey } from "@/config/interpolated-curve"
import { api } from "../api"
import { ChainLogo } from "../components/ChainLogo"
import { CopyHash } from "../components/CopyHash"
import { ExternalLinkIcon } from "../components/InterfaceIcons"
import { TokenIcon } from "../components/TokenIcon"
import {
	describeReferrer,
	formatClockTime,
	formatDate,
	formatTokenAmount,
	shortAddress,
	sqliteUtcToMs,
} from "../lib/format"
import type { ActivityEventDto, BidDto, BidStatsDto, OrderHistoryDto, OrderLeg, OrderSummary } from "../types"

/** Where an order's full record lives; the same page the HyperFX app links to. */
const HYPERFX_ORDER_URL = "https://app.hyperfx.finance/history/details/?id="
const PAGE_SIZE = 20

const CHAIN_META = new Map(INIT_CHAINS.map((meta) => [meta.stateMachineId, meta]))

type OrderRow = OrderHistoryDto["orders"][number]

type Status = { label: string; tone: "" | "ok" | "warn" | "err"; detail?: string }

/**
 * The order's outcome so far. Precedence, not recency: a detection row can be
 * written after the skip that followed it (its token lookups are async), so
 * the strongest event wins regardless of id order.
 */
function statusOf(events: ActivityEventDto[], bids: BidDto[]): Status {
	const find = (type: ActivityEventDto["type"]) => events.find((event) => event.type === type)
	// The amount columns already say what was filled; no volume or profit line here.
	if (find("filled")) return { label: "Filled", tone: "ok" }
	const lost = find("lost")
	if (lost) return { label: "Lost", tone: "warn", detail: lost.reason ? `filled by ${shortAddress(lost.reason)}` : undefined }
	if (find("bid")) {
		// Awaiting the on-chain outcome, unless the bid has since been pulled.
		const latest = bids[0]
		return latest?.retracted ? { label: "Bid retracted", tone: "" } : { label: "Bid placed", tone: "" }
	}
	const executed = find("executed")
	if (executed) {
		return executed.success
			? { label: "Executed", tone: "ok", detail: executed.strategy ? `via ${executed.strategy}` : undefined }
			: { label: "Failed", tone: "err", detail: executed.reason ?? undefined }
	}
	const skipped = find("skipped")
	if (skipped) return { label: "Skipped", tone: "warn", detail: skipped.reason ?? undefined }
	return { label: "Detected", tone: "" }
}

/** Hyperbridge's explorer for the network the filler bids on. */
function hyperbridgeExplorer(network: OrderHistoryDto["network"]): string {
	return network === "testnet" ? "https://gargantua.statescan.io" : "https://nexus.statescan.io"
}

/** A Hyperbridge extrinsic as a time plus a short hash linking to the explorer. */
function ExtrinsicCell(props: { at: string; hash: string | null; explorer: string; label: string; title?: string }) {
	const { at, hash, explorer, label, title } = props
	const ms = sqliteUtcToMs(at)
	return (
		<span className="history-time" title={title}>
			<strong>{Number.isNaN(ms) ? at : formatClockTime(ms)}</strong>
			{hash ? (
				<a
					className="history-extrinsic"
					href={`${explorer}/#/extrinsics/${hash}`}
					target="_blank"
					rel="noreferrer"
					aria-label={`${label} on the Hyperbridge explorer`}
				>
					{shortAddress(hash, 8, 4)}
				</a>
			) : (
				<small>{label}</small>
			)}
		</span>
	)
}

function chainLabelFor(stateMachineId: string, chainLabels?: Record<string, string>): string {
	const chainId = parseChainKey(stateMachineId)
	if (chainId !== null && chainLabels?.[String(chainId)]) return chainLabels[String(chainId)]
	return CHAIN_META.get(stateMachineId)?.label ?? stateMachineId
}

function explorerTxUrl(stateMachineId: string | null, txHash: string | null): string | undefined {
	if (!stateMachineId || !txHash) return undefined
	const explorer = CHAIN_META.get(stateMachineId)?.explorerUrl
	return explorer ? `${explorer}/tx/${txHash}` : undefined
}

function Empty() {
	return (
		<span className="history-referrer" data-empty="true">
			—
		</span>
	)
}

function LegCell(props: { leg: OrderLeg | undefined; chain: string | undefined; chainLabels?: Record<string, string> }) {
	const { leg, chain, chainLabels } = props
	if (!leg || !chain) return <Empty />
	const label = chainLabelFor(chain, chainLabels)
	const symbol =
		leg.symbol ?? (leg.token === "0x0000000000000000000000000000000000000000" ? "native" : shortAddress(leg.token))
	return (
		<span className="history-leg">
			<span className="history-leg-icon" aria-hidden="true">
				<TokenIcon symbol={leg.symbol ?? ""} />
				<ChainLogo label={label} />
			</span>
			<span className="history-leg-copy">
				<strong>
					<span>{formatTokenAmount(leg.amount, leg.decimals)}</span>
					<span>{symbol}</span>
				</strong>
				<small>{label}</small>
			</span>
		</span>
	)
}

function OrderHistoryRow(props: { row: OrderRow; chainLabels?: Record<string, string>; explorer: string }) {
	const { row, chainLabels, explorer } = props
	const status = statusOf(row.events, row.bids)
	// The latest bid: re-bids after a retraction replace earlier ones.
	const bid: BidDto | undefined = row.bids[0]
	const summary: OrderSummary | null = row.events.find((event) => event.order)?.order ?? null
	const detectedAt = row.events.reduce((earliest, event) => Math.min(earliest, event.ts), Number.POSITIVE_INFINITY)
	// The fill's tx hash: from the on-chain fill when observed, else from a direct
	// fill attempt (a UserOp hash resolves on the explorer's /tx page too). A bid's
	// hash is a Hyperbridge extrinsic, which no EVM explorer knows.
	const filled = row.events.find(
		(event) => (event.type === "filled" || event.type === "lost" || event.type === "executed") && event.txHash,
	)
	const fillChain =
		filled?.chainId !== null && filled?.chainId !== undefined ? `EVM-${filled.chainId}` : (summary?.destination ?? null)
	const placedUrl = explorerTxUrl(summary?.source ?? null, summary?.placedTxHash ?? null)
	const fillUrl = explorerTxUrl(fillChain, filled?.txHash ?? null)

	return (
		<tr>
			<td>
				{summary?.referrer ? (
					<span className="history-referrer">
						<CopyHash value={summary.referrer} copyLabel="Copy referrer tag">
							{describeReferrer(summary.referrer)}
						</CopyHash>
					</span>
				) : (
					<Empty />
				)}
			</td>
			<td>
				<span className="history-status">
					<span className={`badge ${status.tone}`}>{status.label}</span>
					{status.detail && <small title={status.detail}>{status.detail}</small>}
					{/* Rows recorded before order summaries existed: keep the id visible. */}
					{!summary && <small title={row.orderId}>order {shortAddress(row.orderId, 8, 4)}</small>}
				</span>
			</td>
			<td>
				<LegCell leg={summary?.inputs[0]} chain={summary?.source} chainLabels={chainLabels} />
			</td>
			<td>
				<LegCell leg={summary?.outputs[0]} chain={summary?.destination} chainLabels={chainLabels} />
			</td>
			<td>
				{bid ? (
					bid.success ? (
						<ExtrinsicCell at={bid.createdAt} hash={bid.extrinsicHash} explorer={explorer} label="Bid" />
					) : (
						<span className="history-bids" title={bid.error ?? undefined}>
							<strong>Failed</strong>
							<small data-tone="err">{bid.error ?? ""}</small>
						</span>
					)
				) : (
					<Empty />
				)}
			</td>
			<td>
				{/* Only a retraction that went on chain. A bid closed out because the pallet no
				    longer had it (our fill consumed it, or it never landed) is marked retracted
				    without an extrinsic, and there is nothing to link. */}
				{bid?.retracted && bid.retractExtrinsicHash ? (
					<ExtrinsicCell
						at={bid.retractedAt ?? bid.createdAt}
						hash={bid.retractExtrinsicHash}
						explorer={explorer}
						label="Retraction"
					/>
				) : (
					<Empty />
				)}
			</td>
			<td>
				{summary ? (
					<span className="history-user">
						<CopyHash value={summary.user} copyLabel="Copy user address">
							{shortAddress(summary.user)}
						</CopyHash>
					</span>
				) : (
					<Empty />
				)}
			</td>
			<td>
				<span className="history-time">
					<strong>{formatClockTime(detectedAt)}</strong>
					<small>{formatDate(detectedAt)}</small>
				</span>
			</td>
			<td>
				<span className="history-links">
					<a
						href={`${HYPERFX_ORDER_URL}${row.orderId}`}
						target="_blank"
						rel="noreferrer"
						title="Open order on HyperFX"
						aria-label="Open order on HyperFX"
					>
						<ExternalLinkIcon aria-hidden="true" />
					</a>
					{placedUrl && (
						<a
							href={placedUrl}
							target="_blank"
							rel="noreferrer"
							title="Placement transaction on the block explorer"
							aria-label="Placement transaction on the block explorer"
						>
							<svg
								viewBox="0 0 16 16"
								fill="none"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeLinecap="round"
								aria-hidden="true"
							>
								<path d="M8 2.5v11M3.5 7 8 2.5 12.5 7" />
							</svg>
						</a>
					)}
					{fillUrl && (
						<a
							href={fillUrl}
							target="_blank"
							rel="noreferrer"
							title="Fill transaction on the block explorer"
							aria-label="Fill transaction on the block explorer"
						>
							<svg
								viewBox="0 0 16 16"
								fill="none"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeLinecap="round"
								aria-hidden="true"
							>
								<path d="M8 13.5v-11M3.5 9 8 13.5 12.5 9" />
							</svg>
						</a>
					)}
				</span>
			</td>
		</tr>
	)
}

/** Page numbers with the current page's neighbours, the ends, and ellipses between. */
function pageNumbers(current: number, last: number): Array<number | "…"> {
	if (last <= 7) return Array.from({ length: last }, (_, index) => index + 1)
	const wanted = new Set([1, 2, last - 1, last, current - 1, current, current + 1])
	const pages = [...wanted].filter((page) => page >= 1 && page <= last).sort((a, b) => a - b)
	const out: Array<number | "…"> = []
	for (const page of pages) {
		const previous = out[out.length - 1]
		if (typeof previous === "number" && page - previous > 1) out.push("…")
		out.push(page)
	}
	return out
}

function Pager(props: { page: number; pageSize: number; total: number; onPage: (page: number) => void }) {
	const { page, pageSize, total, onPage } = props
	const last = Math.max(1, Math.ceil(total / pageSize))
	const from = total === 0 ? 0 : (page - 1) * pageSize + 1
	const to = Math.min(total, page * pageSize)
	return (
		<nav className="history-pager" aria-label="Order history pages">
			<small>{total === 0 ? "No orders" : `Showing ${from}–${to} of ${total.toLocaleString()} orders`}</small>
			<div className="history-pager-pages">
				<button type="button" disabled={page <= 1} onClick={() => onPage(page - 1)} aria-label="Previous page">
					‹
				</button>
				{pageNumbers(page, last).map((entry, index) =>
					entry === "…" ? (
						// biome-ignore lint/suspicious/noArrayIndexKey: ellipses have no identity beyond their slot
						<span key={`gap-${index}`}>…</span>
					) : (
						<button
							type="button"
							key={entry}
							data-active={entry === page}
							aria-current={entry === page ? "page" : undefined}
							onClick={() => onPage(entry)}
						>
							{entry}
						</button>
					),
				)}
				<button type="button" disabled={page >= last} onClick={() => onPage(page + 1)} aria-label="Next page">
					›
				</button>
			</div>
		</nav>
	)
}

export function Activity(props: { chainLabels?: Record<string, string> }) {
	const [page, setPage] = useState(1)
	const [history, setHistory] = useState<OrderHistoryDto>()
	const [stats, setStats] = useState<BidStatsDto | null>(null)
	const [live, setLive] = useState(false)
	const [error, setError] = useState<string>()
	const refreshTimer = useRef<number | undefined>(undefined)

	const load = useCallback(async (target: number) => {
		try {
			const [feed, bidFeed] = await Promise.all([
				api.get<OrderHistoryDto>(`/api/activity/history?page=${target}&pageSize=${PAGE_SIZE}`),
				api.get<{ stats: BidStatsDto | null }>("/api/activity/bids?limit=1"),
			])
			setHistory(feed)
			setStats(bidFeed.stats)
			setError(undefined)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])

	useEffect(() => {
		void load(page)
	}, [load, page])

	// Live tail: any new row re-reads the page being viewed, coalescing bursts
	// (a detection is followed within a second by its skip or fill).
	useEffect(() => {
		const source = new EventSource("/api/events")
		source.onopen = () => setLive(true)
		source.onerror = () => setLive(false)
		source.onmessage = () => {
			window.clearTimeout(refreshTimer.current)
			refreshTimer.current = window.setTimeout(() => void load(page), 400)
		}
		return () => {
			source.close()
			window.clearTimeout(refreshTimer.current)
		}
	}, [load, page])

	const orders = history?.orders ?? []
	const other = history?.other ?? []

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
						<h2>Order history</h2>
					</div>
					<span className="row" style={{ gap: "0.5rem" }}>
						{stats?.pendingRetraction ? (
							<span className="badge warn">{stats.pendingRetraction} retractions pending</span>
						) : null}
						<span className={`badge ${live ? "ok" : "warn"}`}>{live ? "live" : "reconnecting…"}</span>
					</span>
				</div>
				{history && orders.length === 0 && (
					<p className="operator-empty">
						{history.total === 0 ? "No orders yet. Rows appear as orders are detected." : "No orders on this page."}
					</p>
				)}
				{orders.length > 0 && (
					<div style={{ overflowX: "auto" }}>
						<table className="history-table">
							<thead>
								<tr>
									<th>Referrer</th>
									<th>Status</th>
									<th>Amount in</th>
									<th>Amount out</th>
									<th>Bid placed</th>
									<th>Retracted</th>
									<th>User</th>
									<th>Placed</th>
									<th aria-label="Links" />
								</tr>
							</thead>
							<tbody>
								{orders.map((row) => (
									<OrderHistoryRow
										key={row.orderId}
										row={row}
										chainLabels={props.chainLabels}
										explorer={hyperbridgeExplorer(history?.network ?? "mainnet")}
									/>
								))}
							</tbody>
						</table>
					</div>
				)}
				{history && history.total > 0 && (
					<Pager page={history.page} pageSize={history.pageSize} total={history.total} onPage={setPage} />
				)}
				{page === 1 && other.length > 0 && (
					<div className="history-misc" aria-label="Other events">
						<ul>
							{other.map((event) => (
								<li key={event.id}>
									<span className={`badge ${event.success === false ? "err" : ""}`}>{event.type}</span>
									<span>{event.reason ?? (event.success ? "ok" : "")}</span>
									<span>{formatClockTime(event.ts)}</span>
								</li>
							))}
						</ul>
					</div>
				)}
			</section>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
