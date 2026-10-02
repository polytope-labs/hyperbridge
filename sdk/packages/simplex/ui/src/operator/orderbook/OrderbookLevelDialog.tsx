import { CopyHash } from "../../components/CopyHash"
import { CloseIcon } from "../../components/InterfaceIcons"
import {
	ResponsiveDialog,
	ResponsiveDialogClose,
	ResponsiveDialogContent,
	ResponsiveDialogDescription,
	ResponsiveDialogTitle,
} from "../../components/ui/ResponsiveDialog"
import type { OrderbookLevelOrder } from "../../types"
import { formatFixed, formatPrice, formatSize, type DepthLevel } from "./orderbookModel"
import { useLevelOrders } from "./useLevelOrders"

/** Time to expiry at the coarsest two units that still say something: days and hours, then hours and minutes. */
export function formatExpiry(expiresAt: string, now: number): string {
	const remaining = Math.floor((Date.parse(expiresAt) - now) / 60_000)
	if (!Number.isFinite(remaining)) return "—"
	if (remaining <= 0) return "Expired"
	const days = Math.floor(remaining / 1_440)
	const hours = Math.floor((remaining % 1_440) / 60)
	const minutes = remaining % 60
	if (days) return `Expires in ${days}d ${hours}h`
	if (hours) return `Expires in ${hours}h ${minutes}m`
	return `Expires in ${minutes}m`
}

function shortAddress(address: string): string {
	return address.length > 12 ? `${address.slice(0, 6)}…${address.slice(-4)}` : address
}

/** Best price first, as the ladder orders its levels. */
function bestFirst(orders: OrderbookLevelOrder[], side: "BID" | "ASK"): OrderbookLevelOrder[] {
	return [...orders].sort((left, right) => {
		const a = BigInt(left.price)
		const b = BigInt(right.price)
		if (a === b) return 0
		return (a > b ? -1 : 1) * (side === "BID" ? 1 : -1)
	})
}

function plural(count: number, noun: string): string {
	return `${count} ${count === 1 ? noun : `${noun}s`}`
}

function OrderRow(props: {
	order: OrderbookLevelOrder
	decimals: number
	quote: string
	sizeUnit: string
	now: number
	chainLabel: (id: string) => string
}) {
	const { order, decimals, quote, sizeUnit, now, chainLabel } = props
	return (
		<li className="orderbook-level-order">
			<div className="orderbook-level-order-main">
				<CopyHash value={order.solver} copyLabel="Copy solver address">
					{shortAddress(order.solver)}
				</CopyHash>
				<p className="orderbook-level-order-size">
					{formatFixed(order.advertisedSize, 2)} <span>{sizeUnit}</span>
				</p>
			</div>
			<div className="orderbook-level-order-meta">
				<span>{formatExpiry(order.expiresAt, now)}</span>
				<span className="orderbook-level-order-price">
					@ {formatPrice(order.price, decimals)} {quote}
				</span>
			</div>
			{order.resized ? (
				<p className="orderbook-level-order-resized">
					Resized to the solver's balance from {formatFixed(order.quotedAmount, 2)} {sizeUnit} quoted
				</p>
			) : null}
			<p className="orderbook-level-order-sources">
				<span>Accepts swaps from</span> {order.acceptedSources.map(chainLabel).join(", ")}
			</p>
		</li>
	)
}

export function OrderbookLevelDialog(props: {
	book: string
	level: DepthLevel | null
	sourceChain: string
	base: string
	quote: string
	decimals: number
	chainLabel: (id: string) => string
	onClose: () => void
}) {
	const { book, level, sourceChain, base, quote, decimals, chainLabel, onClose } = props
	const state = useLevelOrders(book, level, sourceChain)
	if (!level) return null

	const isBid = level.side === "BID"
	// An order's sizes are in the token its side pays out: quote for bids, base for asks.
	const sizeUnit = isBid ? quote : base
	const route = sourceChain ? `from ${chainLabel(sourceChain)}` : "from any chain"
	const now = Date.now()

	return (
		<ResponsiveDialog open onOpenChange={(open) => (open ? undefined : onClose())}>
			<ResponsiveDialogContent className="orderbook-level-dialog" overlayClassName="dialog-overlay">
				<header className="orderbook-level-dialog-header" data-side={level.side}>
					<div>
						<span className="orderbook-level-dialog-side">{isBid ? "Buy level" : "Sell level"}</span>
						<ResponsiveDialogTitle>
							{formatPrice(level.price, decimals)} <span>{quote}</span>
						</ResponsiveDialogTitle>
						<ResponsiveDialogDescription>
							Fills on {chainLabel(level.fillChain)}, swaps {route}
						</ResponsiveDialogDescription>
					</div>
					<ResponsiveDialogClose>
						<button type="button" className="icon-button market-dialog-close" aria-label="Close">
							<CloseIcon aria-hidden="true" />
						</button>
					</ResponsiveDialogClose>
				</header>

				<dl className="orderbook-level-dialog-stats">
					<div>
						<dt>Level size</dt>
						<dd>
							{formatSize(level.baseSize)} <span>{base}</span>
						</dd>
					</div>
					<div>
						<dt>Orders</dt>
						<dd>{level.orderCount}</dd>
					</div>
					<div>
						<dt>Solvers</dt>
						<dd>{level.solverCount}</dd>
					</div>
				</dl>

				<div className="orderbook-level-dialog-body">
					{state.error ? (
						<p className="orderbook-level-dialog-note" role="alert">
							{state.error}
						</p>
					) : !state.orders ? (
						<p className="orderbook-level-dialog-note" role="status">
							Loading orders…
						</p>
					) : state.orders.length === 0 ? (
						<p className="orderbook-level-dialog-note">
							These orders have left the book since the ladder was drawn.
						</p>
					) : (
						<>
							<p className="orderbook-level-dialog-caption">
								{plural(state.orders.length, "order")}, best price first. Sizes in {sizeUnit}.
							</p>
							<ul className="orderbook-level-orders" aria-label="Orders at this level">
								{bestFirst(state.orders, level.side).map((order) => (
									<OrderRow
										key={order.commitment}
										order={order}
										decimals={decimals}
										quote={quote}
										sizeUnit={sizeUnit}
										now={now}
										chainLabel={chainLabel}
									/>
								))}
							</ul>
						</>
					)}
				</div>
			</ResponsiveDialogContent>
		</ResponsiveDialog>
	)
}
