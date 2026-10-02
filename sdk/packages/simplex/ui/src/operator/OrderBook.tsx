import { useEffect, useId, useState, type ReactNode } from "react"
import { AppSelect, type AppSelectOption } from "../components/AppSelect"
import { ChainLogo } from "../components/ChainLogo"
import { TokenPairIcons } from "../components/TokenIcon"
import { useOrderbookBooks } from "./limitOrders/useLimitOrders"
import { OrderbookLadder } from "./orderbook/OrderbookLadder"
import {
	formatPrice,
	formatRelativeTime,
	formatSize,
	formatSpreadBps,
	topOfBook,
	type TopOfBook,
} from "./orderbook/orderbookModel"
import { useOrderbookSnapshot } from "./orderbook/useOrderbookSnapshot"
import "../styles/orderbook.css"

/** The orderbook spells tokens its own way (`cNGN`); the page uses one spelling throughout. */
const displaySymbol = (symbol: string) => symbol.toUpperCase()

export function OrderBook() {
	const metadata = useOrderbookBooks()
	const [selectedBook, setSelectedBook] = useState("")
	const [sourceChain, setSourceChain] = useState("")
	const [fillChain, setFillChain] = useState("")
	const book = metadata.books.find((item) => item.id === selectedBook) ?? metadata.books[0]
	const state = useOrderbookSnapshot(book?.id ?? "", sourceChain, fillChain)
	const snapshot = state.snapshot
	const top = snapshot ? topOfBook(snapshot) : undefined
	const base = displaySymbol(book?.base ?? "")
	const quote = displaySymbol(book?.quote ?? "")
	const chainLabel = (id: string) => metadata.chains.find((chain) => chain.id === id)?.name ?? id
	const chainOptions: AppSelectOption[] = [
		{ value: "", label: "All Chains" },
		...metadata.chains.map((chain) => ({
			value: chain.id,
			label: chain.name,
			leading: (
				<span className="orderbook-chain-icon" aria-hidden="true">
					<ChainLogo label={chain.name} />
				</span>
			),
		})),
	]
	const pairOptions: AppSelectOption[] = metadata.books.map((item) => ({
		value: item.id,
		label: `${displaySymbol(item.base)} / ${displaySymbol(item.quote)}`,
		leading: (
			<span className="orderbook-pair-icons">
				<TokenPairIcons tokenA={item.base} tokenB={item.quote} />
			</span>
		),
	}))

	if (metadata.error) {
		return (
			<p className="error" role="alert">
				{metadata.error}
				<button type="button" onClick={metadata.reload}>
					Retry pairs
				</button>
			</p>
		)
	}
	if (metadata.loading) return <OrderbookSkeleton />
	if (!book) return <EmptyState label="The order book has no pairs yet." />

	const filters = (
		<div className="orderbook-filters">
			<RouteFilter label="Pair" value={book.id} options={pairOptions} onChange={setSelectedBook} />
			<RouteFilter label="From" value={sourceChain} options={chainOptions} onChange={setSourceChain} />
			<RouteFilter label="To" value={fillChain} options={chainOptions} onChange={setFillChain} />
		</div>
	)
	const isEmpty = top ? top.bids.length === 0 && top.asks.length === 0 : false

	// The filters stay mounted through loading and errors: the select just used keeps its focus,
	// and a failing book can still be switched away from.
	const unavailable = Boolean(state.error) && !snapshot
	return (
		<section className="orderbook-view" aria-label="Live order book">
			<section className="orderbook-card orderbook-summary" aria-label="Market summary">
				{filters}
				{unavailable ? (
					<p className="orderbook-empty-copy" role="alert">
						The order book is temporarily unavailable. Retrying automatically.
					</p>
				) : !top ? (
					<StatsSkeleton />
				) : isEmpty ? (
					<p className="orderbook-empty-copy">
						{sourceChain || fillChain
							? "No solver is quoting this pair for that route right now."
							: "No solver is quoting this pair right now."}{" "}
						Orders carry a short expiry, so a book can empty and refill within minutes.
					</p>
				) : (
					<OrderbookStats
						top={top}
						base={base}
						quote={quote}
						updatedAt={state.updatedAt!}
						stale={Boolean(state.error)}
					/>
				)}
			</section>
			{unavailable || isEmpty ? null : top ? (
				<OrderbookLadder
					key={state.key}
					book={book.id}
					sourceChain={sourceChain}
					top={top}
					base={base}
					quote={quote}
					chainLabel={chainLabel}
				/>
			) : (
				<div className="orderbook-card orderbook-skeleton-panel" aria-hidden="true" />
			)}
		</section>
	)
}

function RouteFilter(props: {
	label: string
	value: string
	options: AppSelectOption[]
	onChange: (value: string) => void
}) {
	const labelId = useId()
	return (
		<div className="orderbook-filter">
			<span id={labelId}>{props.label}</span>
			<AppSelect
				value={props.value}
				options={props.options}
				onValueChange={props.onChange}
				ariaLabelledBy={labelId}
			/>
		</div>
	)
}

function Stat(props: { label: ReactNode; tone?: "buy" | "sell"; children: ReactNode }) {
	return (
		<div className="orderbook-stat">
			<div className="orderbook-stat-label">{props.label}</div>
			<p className="orderbook-stat-value" data-tone={props.tone}>
				{props.children}
			</p>
		</div>
	)
}

function LiveIndicator({ updatedAt, stale }: { updatedAt: number; stale: boolean }) {
	const [now, setNow] = useState(() => Date.now())
	useEffect(() => {
		const timer = setInterval(() => setNow(Date.now()), 1_000)
		return () => clearInterval(timer)
	}, [])
	return (
		<span className="orderbook-live" data-stale={stale || undefined}>
			<span aria-hidden="true" />
			{stale ? "Stale" : "Live"} · {formatRelativeTime(updatedAt, now)}
		</span>
	)
}

function OrderbookStats(props: { top: TopOfBook; base: string; quote: string; updatedAt: number; stale: boolean }) {
	const { top, base, quote, updatedAt, stale } = props
	const unit = (symbol: string) => <span className="orderbook-unit">{symbol}</span>

	return (
		<div className="orderbook-stats">
			<Stat
				label={
					<>
						<p>Mid price</p>
						<LiveIndicator updatedAt={updatedAt} stale={stale} />
					</>
				}
			>
				{formatPrice(top.mid, top.priceDecimals)}
				{unit(quote)}
			</Stat>
			<Stat label="Best buy" tone="buy">
				{formatPrice(top.bestBid, top.priceDecimals)}
				{unit(quote)}
			</Stat>
			<Stat label="Best sell" tone="sell">
				{formatPrice(top.bestAsk, top.priceDecimals)}
				{unit(quote)}
			</Stat>
			<Stat label="Spread">
				{top.crossed ? (
					"crossed"
				) : top.spread ? (
					<>
						{formatPrice(top.spread, top.priceDecimals)}
						<span className="orderbook-unit orderbook-unit-wide">{formatSpreadBps(top.spreadBps)}</span>
					</>
				) : (
					"—"
				)}
			</Stat>
			{/* Backed liquidity, not advertised size: orders sharing one balance count once. */}
			<Stat label="Buy liquidity" tone="buy">
				{formatSize(top.bidLiquidity)}
				{unit(quote)}
			</Stat>
			<Stat label="Sell liquidity" tone="sell">
				{formatSize(top.askLiquidity)}
				{unit(base)}
			</Stat>
		</div>
	)
}

function EmptyState({ label }: { label: string }) {
	return (
		<div className="orderbook-card orderbook-empty-state">
			<p>{label}</p>
		</div>
	)
}

/** Sizes in px, mirroring the content each block stands in for. */
function Pulse({ width, height, gap = 0 }: { width: number; height: number; gap?: number }) {
	return <span aria-hidden="true" className="orderbook-pulse" style={{ width, height, marginTop: gap }} />
}

function StatsSkeleton({ announce = true }: { announce?: boolean }) {
	return (
		<div
			className="orderbook-stats"
			role={announce ? "status" : undefined}
			aria-label={announce ? "Loading prices" : undefined}
			aria-hidden={announce ? undefined : true}
		>
			{[0, 1, 2, 3, 4, 5].map((index) => (
				<div key={index}>
					<Pulse width={96} height={12} />
					<Pulse width={112} height={20} gap={8} />
				</div>
			))}
		</div>
	)
}

/** Before the pairs arrive there are no filters to keep mounted, so the whole page stands in. */
function OrderbookSkeleton() {
	return (
		<div aria-label="Loading order book" role="status" className="orderbook-skeleton">
			<div className="orderbook-card orderbook-summary">
				<div className="orderbook-filters">
					<Pulse width={224} height={40} />
					<Pulse width={224} height={40} />
					<Pulse width={224} height={40} />
				</div>
				<StatsSkeleton announce={false} />
			</div>
			<div className="orderbook-card orderbook-skeleton-panel" />
		</div>
	)
}
