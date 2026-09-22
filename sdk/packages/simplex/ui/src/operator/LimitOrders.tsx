import { useEffect, useState } from "react"
import { ChevronRightIcon } from "../components/InterfaceIcons"
import { OperatorSheet } from "../components/OperatorSheet"
import { Pager } from "../components/Pager"
import { TokenPairIcons } from "../components/TokenIcon"
import { INIT_CHAINS } from "@/cli/init/chains"
import { formatDate, sqliteUtcToMs } from "../lib/format"
import type { LimitOrder } from "../types"
import { CreateLimitOrderForm } from "./limitOrders/CreateLimitOrderForm"
import { available, describeRate, fromScaled, legs, statusOf } from "./limitOrders/limitOrderModel"
import { type LimitOrderFills, useLimitOrders } from "./limitOrders/useLimitOrders"

/** Orders per page, live and closed each: a solver re-posting all day builds a long closed list. */
const PAGE_SIZE = 10

interface LimitOrdersProps {
	/** Chain ids the filler runs. A limit order names them as state machine ids. */
	chains: number[]
	chainLabels?: Record<string, string>
	/** Symbols the operator can trade, from what the filler knows how to send. */
	symbols: string[]
}

/**
 * The operator's resting orders: what simplex is offering, on what terms, and
 * what is left of each.
 *
 * This is where prices live now. A limit order names two amounts and simplex
 * fills at the rate they imply until the order runs out, so the page is a book
 * of standing offers rather than a curve to shape.
 */
export function LimitOrders({ chains, chainLabels, symbols }: LimitOrdersProps) {
	const { orders, loading, error, create, cancel, withFills, reload } = useLimitOrders()
	const [creating, setCreating] = useState(false)
	const [selected, setSelected] = useState<LimitOrder>()
	const [detail, setDetail] = useState<LimitOrderFills>()
	const [busy, setBusy] = useState(false)
	const [actionError, setActionError] = useState<string>()
	const [livePage, setLivePage] = useState(1)
	const [closedPage, setClosedPage] = useState(1)

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

	const live = orders.filter((order) => order.status === "open" || order.status === "resizing")
	const closed = orders.filter((order) => order.status !== "open" && order.status !== "resizing")
	// A page that emptied under a refresh (orders filled or cancelled) falls back to the last one.
	const liveLast = Math.max(1, Math.ceil(live.length / PAGE_SIZE))
	const closedLast = Math.max(1, Math.ceil(closed.length / PAGE_SIZE))
	useEffect(() => {
		if (livePage > liveLast) setLivePage(liveLast)
		if (closedPage > closedLast) setClosedPage(closedLast)
	}, [livePage, liveLast, closedPage, closedLast])
	const livePageOrders = live.slice((livePage - 1) * PAGE_SIZE, livePage * PAGE_SIZE)
	const closedPageOrders = closed.slice((closedPage - 1) * PAGE_SIZE, closedPage * PAGE_SIZE)

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

				<div className="operator-market-list">
					{livePageOrders.map((order) => (
						<LimitOrderRow key={order.id} order={order} chainLabel={chainLabel} onOpen={() => void open(order)} />
					))}
					{live.length === 0 && !loading ? (
						<p className="operator-empty">
							Nothing is resting on the orderbook. Post a limit order to say what simplex will pay.
						</p>
					) : null}
				</div>
				{live.length > PAGE_SIZE ? (
					<Pager page={livePage} pageSize={PAGE_SIZE} total={live.length} noun="orders" onPage={setLivePage} />
				) : null}

				{closed.length > 0 ? (
					<>
						<div className="operator-section-heading">
							<div>
								<span className="eyebrow">Closed</span>
							</div>
						</div>
						<div className="operator-market-list">
							{closedPageOrders.map((order) => (
								<LimitOrderRow
									key={order.id}
									order={order}
									chainLabel={chainLabel}
									onOpen={() => void open(order)}
								/>
							))}
						</div>
						{closed.length > PAGE_SIZE ? (
							<Pager
								page={closedPage}
								pageSize={PAGE_SIZE}
								total={closed.length}
								noun="orders"
								onPage={setClosedPage}
							/>
						) : null}
					</>
				) : null}
			</section>

			<OperatorSheet
				open={creating}
				onClose={() => setCreating(false)}
				wide
				title="New limit order"
				description="Say what you take in and what you pay out. Simplex fills at the rate the two imply."
			>
				<CreateLimitOrderForm
					symbols={symbols}
					chains={chainOptions.map((chain) => chain.stateMachineId)}
					chainLabel={chainLabel}
					create={create}
					onCreated={() => setCreating(false)}
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

function LimitOrderRow(props: { order: LimitOrder; chainLabel: (id: string) => string; onOpen: () => void }) {
	const { order, chainLabel, onOpen } = props
	const status = statusOf(order)
	const { input, output } = legs(order)

	return (
		<button type="button" className="operator-market-row" onClick={onOpen}>
			<TokenPairIcons tokenA={input} tokenB={output} />
			<span className="operator-market-copy">
				<strong>
					{fromScaled(order.remaining)} {output} left at {describeRate(order)}
				</strong>
				<small>
					takes {input} on {chainLabel(order.fillChain)}
				</small>
			</span>
			<span className={`badge ${status.tone}`}>{status.label}</span>
			<ChevronRightIcon aria-hidden="true" />
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
						{fromScaled(order.reserved)} {output}, leaving {fromScaled(available(order).toString())} to draw on
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
					<ol className="limit-order-fills">
						<li className="limit-order-fills-head" aria-hidden="true">
							<span>When</span>
							<span>Paid out</span>
							<span>Transaction</span>
						</li>
						{fills.map((fill) => {
							const explorer = EXPLORER_BY_CHAIN.get(order.fillChain)
							const hash = fill.transactionHash
							return (
								<li key={fill.id}>
									<time>{formatDate(sqliteUtcToMs(fill.filledAt))}</time>
									<strong>
										{fromScaled(fill.amount)} <small>{output}</small>
									</strong>
									{hash && explorer ? (
										<a href={`${explorer}/tx/${hash}`} target="_blank" rel="noreferrer">
											{hash.slice(0, 8)}…{hash.slice(-6)}
										</a>
									) : (
										<span className="limit-order-fills-none">—</span>
									)}
								</li>
							)
						})}
					</ol>
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
