import { type ReactNode, useState } from "react"
import { ChainLogo } from "../../components/ChainLogo"
import { formatPrice, formatSize, formatSpreadBps, type DepthLevel, type TopOfBook } from "./orderbookModel"

/** Deep books are the goal, but a ladder taller than this stops being one. */
const COLLAPSED_LEVEL_COUNT = 8
const COLUMN_COUNT = 6

function LevelRow(props: { level: DepthLevel; decimals: number; quote: string; chainLabel: (id: string) => string }) {
	const { level, decimals, quote, chainLabel } = props
	const price = formatPrice(level.price, decimals)
	const chain = chainLabel(level.fillChain)

	return (
		<div role="row" className="orderbook-grid orderbook-level" data-side={level.side}>
			{/* The depth bar runs from the price column, so size and depth read in one sweep. */}
			<span aria-hidden="true" className="orderbook-level-bar" style={{ width: `${Math.max(1, level.depthRatio)}%` }} />
			<span
				role="cell"
				className="orderbook-level-price"
				title={`Worst price ${formatPrice(level.worstPrice, decimals)} ${quote}`}
			>
				{price}
			</span>
			<span role="cell" className="orderbook-col-extra orderbook-chain">
				<span className="orderbook-chain-icon" aria-hidden="true">
					<ChainLogo label={chain} />
				</span>
				{chain}
			</span>
			<span
				role="cell"
				className="orderbook-col-extra orderbook-num orderbook-muted"
				title={`${level.orderCount} ${level.orderCount === 1 ? "order" : "orders"} from ${level.solverCount} ${level.solverCount === 1 ? "solver" : "solvers"}`}
			>
				{level.orderCount} · {level.solverCount}
			</span>
			<span role="cell" className="orderbook-num">
				{formatSize(level.baseSize)}
			</span>
			<span role="cell" className="orderbook-col-extra orderbook-num">
				{formatSize(level.quoteSize)}
			</span>
			<span role="cell" className="orderbook-num">
				{formatSize(level.cumulativeBase)}
			</span>
		</div>
	)
}

/**
 * Sides are named as the orders are: a bid holds the quote and buys the base, an ask sells it.
 * Headings, the mid, and empty sides are full-width rows so the table stays a valid grid of rows.
 */
function SideHeading({ count, side }: { count: number; side: "BID" | "ASK" }) {
	return (
		<div role="row" className="orderbook-side-heading" data-side={side}>
			<span role="rowheader" aria-colspan={COLUMN_COUNT}>
				<span>{side === "BID" ? "Buy" : "Sell"}</span> <span className="orderbook-side-count">{count}</span>
			</span>
		</div>
	)
}

function EmptySide({ children }: { children: ReactNode }) {
	return (
		<div role="row">
			<p role="cell" aria-colspan={COLUMN_COUNT} className="orderbook-empty-side">
				{children}
			</p>
		</div>
	)
}

export function OrderbookLadder(props: {
	top: TopOfBook
	base: string
	quote: string
	chainLabel: (id: string) => string
}) {
	const { top, base, quote, chainLabel } = props
	const [expanded, setExpanded] = useState(false)
	const hiddenCount =
		Math.max(0, top.asks.length - COLLAPSED_LEVEL_COUNT) + Math.max(0, top.bids.length - COLLAPSED_LEVEL_COUNT)
	const asks = expanded ? top.asks : top.asks.slice(0, COLLAPSED_LEVEL_COUNT)
	const bids = expanded ? top.bids : top.bids.slice(0, COLLAPSED_LEVEL_COUNT)
	const row = (level: DepthLevel) => (
		<LevelRow key={level.id} level={level} decimals={top.priceDecimals} quote={quote} chainLabel={chainLabel} />
	)

	return (
		<section className="orderbook-card orderbook-ladder" aria-label="Order book ladder">
			<div className="orderbook-ladder-head">
				<h2 className="orderbook-card-title orderbook-card-title-lg">Order book</h2>
				<p className="orderbook-caption">{top.bidOrderCount + top.askOrderCount} live orders</p>
			</div>

			<div role="table" aria-label={`${base}/${quote} bids and asks`}>
				<div role="row" className="orderbook-grid orderbook-columns">
					<span role="columnheader">Price ({quote})</span>
					<span role="columnheader" className="orderbook-col-extra">
						Fill chain
					</span>
					<span role="columnheader" className="orderbook-col-extra">
						Orders · solvers
					</span>
					<span role="columnheader">Size ({base})</span>
					<span role="columnheader" className="orderbook-col-extra">
						Total ({quote})
					</span>
					<span role="columnheader">Sum ({base})</span>
				</div>

				<SideHeading count={top.asks.length} side="ASK" />
				{asks.length ? (
					// Highest sell first, so the best sell price sits against the mid.
					<div role="rowgroup">{[...asks].reverse().map(row)}</div>
				) : (
					<EmptySide>Nobody is selling {base} on this book.</EmptySide>
				)}

				<div role="row" className="orderbook-mid">
					<span role="cell" aria-colspan={COLUMN_COUNT} className="orderbook-mid-cell">
						<span className="orderbook-muted">Mid</span>
						<span className="orderbook-num">{formatPrice(top.mid, top.priceDecimals)}</span>
						<span className="orderbook-mid-bps">
							Spread {top.crossed ? "crossed" : top.spread ? formatSpreadBps(top.spreadBps) : "—"}
						</span>
					</span>
				</div>

				<SideHeading count={top.bids.length} side="BID" />
				{bids.length ? (
					<div role="rowgroup">{bids.map(row)}</div>
				) : (
					<EmptySide>Nobody is buying {base} on this book.</EmptySide>
				)}
			</div>

			{hiddenCount > 0 ? (
				<button
					type="button"
					className="orderbook-expand"
					aria-expanded={expanded}
					onClick={() => setExpanded((value) => !value)}
				>
					{expanded ? "Show fewer levels" : `Show ${hiddenCount} more levels`}
				</button>
			) : (
				<div className="orderbook-ladder-foot" />
			)}
		</section>
	)
}
