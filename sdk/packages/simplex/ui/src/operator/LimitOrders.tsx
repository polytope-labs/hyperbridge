import { type CSSProperties, useState } from "react"
import { OperatorSheet } from "../components/OperatorSheet"
import { Pager } from "../components/Pager"
import { PillTabs } from "../components/PillTabs"
import { TokenPairIcons } from "../components/TokenIcon"
import { INIT_CHAINS } from "@/cli/init/chains"
import { formatDate, sqliteUtcToMs } from "../lib/format"
import type { BalanceSnapshot, LimitOrder, LimitOrderSide } from "../types"
import { CreateLimitOrderForm } from "./limitOrders/CreateLimitOrderForm"
import {
	describeProgress,
	describeRate,
	fromScaled,
	legs,
	type OrderTab,
	progressOf,
	rateParts,
	rowBadge,
	sideLabel,
	statusOf,
	tabOf,
} from "./limitOrders/limitOrderModel"
import { type LimitOrderFills, useLimitOrders, useOrderbookBooks } from "./limitOrders/useLimitOrders"

/** Orders per page, in every list: a solver re-posting all day builds a long filled list. */
const PAGE_SIZE = 10

/** Buys and sells are never listed together: a rate reads one way round within a list. */
const SIDES: ReadonlyArray<{ value: LimitOrderSide; label: string; noun: string }> = [
	{ value: "BID", label: "Buys", noun: "buy" },
	{ value: "ASK", label: "Sells", noun: "sell" },
]

const TABS: ReadonlyArray<{ value: OrderTab; label: string }> = [
	{ value: "live", label: "Live" },
	{ value: "filled", label: "Filled" },
	{ value: "cancelled", label: "Cancelled" },
]

/** What an empty list says, for a side named as "buy" or "sell". */
const EMPTY: Record<OrderTab, (noun: string) => string> = {
	live: (noun) => `No ${noun} orders are resting on the orderbook. Post a limit order to say what simplex will pay.`,
	filled: (noun) => `No ${noun} orders have filled yet.`,
	cancelled: (noun) => `No ${noun} orders have been cancelled, expired or refused.`,
}

interface LimitOrdersProps {
	/** Chain ids the filler runs. A limit order names them as state machine ids. */
	chains: number[]
	chainLabels?: Record<string, string>
	/** Shown beside each chain in the new order's "Fills on" menu. */
	balances?: BalanceSnapshot
}

/**
 * The operator's resting orders: what simplex is offering, on what terms, and
 * what is left of each.
 *
 * This is where prices live now. A limit order names two amounts and simplex
 * fills at the rate they imply until the order runs out, so the page is a book
 * of standing offers rather than a curve to shape.
 */
export function LimitOrders({ chains, chainLabels, balances }: LimitOrdersProps) {
	const { orders, loading, error, create, cancel, withFills, reload } = useLimitOrders()
	// Only the pairs the orderbook keeps, and the chains it registers their tokens on: it refuses
	// an order naming anything else.
	const { books, chains: orderbookChains } = useOrderbookBooks()
	const [creating, setCreating] = useState(false)
	const [selected, setSelected] = useState<LimitOrder>()
	const [detail, setDetail] = useState<LimitOrderFills>()
	const [busy, setBusy] = useState(false)
	const [actionError, setActionError] = useState<string>()
	const [side, setSide] = useState<LimitOrderSide>("BID")
	const [tab, setTab] = useState<OrderTab>("live")
	const [page, setPage] = useState(1)

	// The status feed counts chains by id; an order names them the way the gateway
	// does, so the page speaks both.
	const chainOptions = chains.map((id) => ({
		stateMachineId: `EVM-${id}`,
		label: chainLabels?.[String(id)] ?? `Chain ${id}`,
	}))
	const chainLabel = (stateMachineId: string) =>
		chainOptions.find((chain) => chain.stateMachineId === stateMachineId)?.label ?? stateMachineId

	const open = async (order: LimitOrder) => {
		setSelected(order)
		setDetail(undefined)
		setActionError(undefined)
		// The fills are what make a shrunken `remaining` explicable, and they are a
		// second read: the list itself carries only the order.
		try {
			setDetail(await withFills(order.id))
		} catch {
			setDetail(undefined)
		}
	}

	const cancelSelected = async () => {
		if (!selected || busy) return
		setBusy(true)
		setActionError(undefined)
		try {
			await cancel(selected.id)
			setSelected(undefined)
		} catch (err) {
			setActionError(err instanceof Error ? err.message : "Could not cancel the limit order")
		} finally {
			setBusy(false)
		}
	}

	const count = (ofSide: LimitOrderSide, ofTab: OrderTab) =>
		orders.filter((order) => order.side === ofSide && tabOf(order) === ofTab).length
	const listed = orders.filter((order) => order.side === side && tabOf(order) === tab)
	// A page that emptied under a refresh (orders filled or cancelled) shows the last one instead.
	const current = Math.min(page, Math.max(1, Math.ceil(listed.length / PAGE_SIZE)))
	const pageOrders = listed.slice((current - 1) * PAGE_SIZE, current * PAGE_SIZE)
	const sideNoun = SIDES.find((entry) => entry.value === side)?.noun ?? "limit"

	// Each list pages on its own terms, so changing list starts it from the top.
	const showSide = (next: LimitOrderSide) => {
		setSide(next)
		setPage(1)
	}
	const showTab = (next: OrderTab) => {
		setTab(next)
		setPage(1)
	}
	// The list follows a new order to where it landed, or posting from another list would close
	// the form on nothing. That is Live, unless the orderbook refused it: a refusal is kept under
	// Cancelled, and is shown ahead of the orders posted alongside it. The newest order leads.
	const showCreated = (created: LimitOrder[]) => {
		const order = created.find((entry) => tabOf(entry) !== "live") ?? created[0]
		if (!order) return
		setSide(order.side)
		setTab(tabOf(order))
		setPage(1)
	}

	return (
		<>
			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Pricing</span>
						<h2>Limit orders</h2>
					</div>
					<button type="button" className="primary limit-order-new-button" onClick={() => setCreating(true)}>
						<span aria-hidden="true">+</span>
						New limit order
					</button>
				</div>

				{error ? <p className="error">{error}</p> : null}

				<div className="limit-order-tabs">
					<div className="limit-order-sides" role="group" aria-label="Order side">
						{SIDES.map((entry) => (
							<button
								key={entry.value}
								type="button"
								data-side={entry.value}
								className={`limit-order-side${side === entry.value ? " is-selected" : ""}`}
								aria-pressed={side === entry.value}
								onClick={() => showSide(entry.value)}
							>
								{entry.label}
								<span className="step-count">{count(entry.value, "live")}</span>
							</button>
						))}
					</div>
					<PillTabs
						options={TABS.map((entry) => ({ ...entry, count: count(side, entry.value) }))}
						value={tab}
						onChange={showTab}
						ariaLabel="Order status"
					/>
				</div>

				<div className="limit-order-list">
					{pageOrders.map((order) => (
						<LimitOrderRow key={order.id} order={order} chainLabel={chainLabel} onOpen={() => void open(order)} />
					))}
					{listed.length === 0 && !loading ? <p className="operator-empty">{EMPTY[tab](sideNoun)}</p> : null}
				</div>
				{listed.length > PAGE_SIZE ? (
					<Pager page={current} pageSize={PAGE_SIZE} total={listed.length} noun="orders" onPage={setPage} />
				) : null}
			</section>

			<OperatorSheet
				open={creating}
				onClose={() => setCreating(false)}
				wide
				title="New limit order"
			>
				<CreateLimitOrderForm
					books={books}
					orderbookChains={orderbookChains}
					chains={chainOptions.map((chain) => chain.stateMachineId)}
					chainLabel={chainLabel}
					balances={balances}
					create={create}
					onCreated={(created) => {
						setCreating(false)
						showCreated(created)
					}}
					onCancel={() => setCreating(false)}
				/>
			</OperatorSheet>

			<OperatorSheet
				open={selected !== undefined}
				onClose={() => setSelected(undefined)}
				title={selected ? `${selected.base}/${selected.quote}` : "Limit order"}
				description={selected ? describeRate(selected) : undefined}
			>
				{selected ? (
					<LimitOrderDetail
						order={detail?.order ?? selected}
						fills={detail?.fills ?? []}
						chainLabel={chainLabel}
						busy={busy}
						error={actionError}
						onCancel={() => void cancelSelected()}
						onRefresh={() => void reload()}
					/>
				) : null}
			</OperatorSheet>
		</>
	)
}

/** Each chain's block explorer, for linking a fill to its transaction. */
const EXPLORER_BY_CHAIN = new Map(INIT_CHAINS.map((meta) => [meta.stateMachineId, meta.explorerUrl]))

/**
 * What a row says under its progress: what is left and where it fills while the order is live,
 * and how it ended and when once it is not. A status with something to explain says that too. The
 * date leads a line that carries an explanation, since the row cuts a long one short.
 */
function describeRow(order: LimitOrder, chainLabel: (id: string) => string): string {
	const status = statusOf(order)
	const { input, output } = legs(order)
	const chain = chainLabel(order.fillChain)
	const when = formatDate(sqliteUtcToMs(order.updatedAt))

	if (tabOf(order) === "live") {
		const parts = [`${fromScaled(order.remaining)} ${output} left`]
		if (order.reserved !== "0") parts.push(`${fromScaled(order.reserved)} held by live bids`)
		parts.push(status.detail ?? `takes ${input} on ${chain}`)
		return parts.join(" · ")
	}
	if (order.status === "filled") {
		return status.detail ? `${when} · ${status.detail}` : `closed ${when} · took ${input} on ${chain}`
	}
	if (order.status === "rejected") return `refused ${when} · ${status.detail ?? "the orderbook gave no reason"}`
	return `${fromScaled(order.remaining)} ${output} unfilled · ${status.label.toLowerCase()} ${when}`
}

/**
 * One order as a bar: the fill is how much of its cap has gone out, and the fainter rule past it
 * what live bids are holding. The rate leads and the cap closes the row.
 */
function LimitOrderRow(props: { order: LimitOrder; chainLabel: (id: string) => string; onOpen: () => void }) {
	const { order, chainLabel, onOpen } = props
	const status = statusOf(order)
	const badge = rowBadge(order)
	const rate = rateParts(order)
	const progress = progressOf(order)
	const { input, output } = legs(order)
	const bar = { "--consumed": `${progress.consumed}%`, "--held": `${progress.held}%` } as CSSProperties
	// A problem reported against a live order is what its second line then says.
	const problem = status.tone === "warn" && order.lastError !== null && status.detail === order.lastError

	return (
		<button
			type="button"
			className="limit-order-item"
			data-side={order.side}
			data-tab={tabOf(order)}
			style={bar}
			onClick={onOpen}
		>
			<TokenPairIcons tokenA={input} tokenB={output} />
			<span className="limit-order-item-rate">
				<strong>{rate.figure}</strong>
				<small>{rate.unit}</small>
			</span>
			<span className="limit-order-item-progress">
				<span>
					<strong>{describeProgress(order)}</strong>
					{badge ? <span className={`badge ${badge.tone}`}>{badge.label}</span> : null}
				</span>
				{/* Unbadged, an order on the book with a problem has only this line to say so. */}
				<small data-tone={problem ? "warn" : undefined}>{describeRow(order, chainLabel)}</small>
			</span>
			<span className="limit-order-item-cap">
				<small>Cap</small>
				<strong>
					{fromScaled(order.size)} <span>{output}</span>
				</strong>
			</span>
		</button>
	)
}

function LimitOrderDetail(props: {
	order: LimitOrder
	fills: LimitOrderFills["fills"]
	chainLabel: (id: string) => string
	busy: boolean
	error?: string
	onCancel: () => void
	onRefresh: () => void
}) {
	const { order, fills, chainLabel, busy, error, onCancel, onRefresh } = props
	const status = statusOf(order)
	const { input, output } = legs(order)
	const closed = order.status !== "open" && order.status !== "resizing"

	return (
		<section className="operator-panel-form">
			<dl className="limit-order-facts">
				<div>
					<dt>Status</dt>
					<dd>
						<span className={`badge ${status.tone}`}>{status.label}</span>
						{status.detail ? <small> {status.detail}</small> : null}
					</dd>
				</div>
				<div>
					<dt>Side</dt>
					<dd>
						<span className="limit-order-side-tag" data-side={order.side}>
							{sideLabel(order.side)}
						</span>{" "}
						<small>{order.base}</small>
					</dd>
				</div>
				<div>
					<dt>Rate</dt>
					<dd>{describeRate(order)}</dd>
				</div>
				<div>
					<dt>Pays out</dt>
					<dd>
						{fromScaled(order.remaining)} {output} left of {fromScaled(order.size)}
					</dd>
				</div>
				<div>
					<dt>Held by live bids</dt>
					<dd>
						{fromScaled(order.reserved)} {output}
					</dd>
				</div>
				<div>
					<dt>Takes in</dt>
					<dd>
						{input} on {chainLabel(order.fillChain)}
					</dd>
				</div>
				<div>
					<dt>Accepts swaps from</dt>
					<dd>{order.acceptedSources.map(chainLabel).join(", ")}</dd>
				</div>
			</dl>

			{order.lastError ? <p className="error">{order.lastError}</p> : null}
			{error ? <p className="error">{error}</p> : null}

			<h3>Fills</h3>
			{fills.length === 0 ? (
				<p className="hint">Nothing has filled against this order yet.</p>
			) : (
				<>
					<table className="limit-order-fills">
						<thead>
							<tr>
								<th scope="col">When</th>
								<th scope="col">Paid out</th>
								<th scope="col">Transaction</th>
							</tr>
						</thead>
						<tbody>
							{fills.map((fill) => {
								const explorer = EXPLORER_BY_CHAIN.get(order.fillChain)
								const hash = fill.transactionHash
								return (
									<tr key={fill.id}>
										<td>
											<time>{formatDate(sqliteUtcToMs(fill.filledAt))}</time>
										</td>
										<td>
											<strong>
												{fromScaled(fill.amount)} <small>{output}</small>
											</strong>
										</td>
										<td>
											{hash && explorer ? (
												<a href={`${explorer}/tx/${hash}`} target="_blank" rel="noreferrer">
													{hash.slice(0, 8)}…{hash.slice(-6)}
												</a>
											) : (
												<span className="limit-order-fills-none">—</span>
											)}
										</td>
									</tr>
								)
							})}
						</tbody>
					</table>
					<p className="limit-order-fills-total">
						{fromScaled(fills.reduce((sum, fill) => sum + BigInt(fill.amount), 0n).toString())} {output} paid out
						across {fills.length} {fills.length === 1 ? "fill" : "fills"}
					</p>
				</>
			)}

			<footer className="market-dialog-footer">
				<button type="button" onClick={onRefresh} disabled={busy}>
					Refresh
				</button>
				{!closed ? (
					<button type="button" className="market-delete-button" onClick={onCancel} disabled={busy}>
						{busy ? "Cancelling…" : "Cancel order"}
					</button>
				) : null}
			</footer>
		</section>
	)
}
