import { useState } from "react"
import { Pager } from "../components/Pager"
import { PillTabs } from "../components/PillTabs"
import { TokenPairIcons } from "../components/TokenIcon"
import { formatAmount, formatDate } from "../lib/format"
import type { BookProfit, ProfitabilityDto, ProfitFigures, ProfitPeriod, StartingInventory } from "../types"
import {
	BUCKET_NOUNS,
	bucketLabel,
	describeHoldings,
	describeNoReturn,
	formatChange,
	formatPercent,
	formatRate,
	formatUsd,
	PERIODS,
	toneOf,
} from "./analytics/analyticsModel"
import { ProfitCharts } from "./analytics/ProfitCharts"
import { useProfitability } from "./analytics/useProfitability"
import "../styles/analytics.css"

/** Buckets per page of the table under the charts. */
const PAGE_SIZE = 10

/**
 * What the operator's buys and sells earned, over a period they choose.
 *
 * Profit is the change the period's fills made to what the solver holds, each token valued at its
 * latest rate, and it is compared with the inventory the period began with. That inventory is on
 * record only where a daily snapshot was taken or, for the last seven days, where it can be
 * rebuilt from today's balances, so an older period shows its profit without a return.
 */
export function Analytics() {
	const [period, setPeriod] = useState<ProfitPeriod>("30d")
	const { summary, loading, error } = useProfitability(period)

	return (
		<div className="analytics">
			<PillTabs
				options={PERIODS}
				value={period}
				onChange={setPeriod}
				ariaLabel="Period"
				className="analytics-periods"
			/>
			{error ? <p className="error">{error}</p> : null}
			{summary ? (
				<Summary summary={summary} />
			) : loading ? (
				<p className="operator-empty">Reading your fills…</p>
			) : null}
		</div>
	)
}

/** Where a starting inventory came from, in a few words under its figure. */
function describeSource(inventory: StartingInventory): string {
	return inventory.source === "snapshot"
		? `From the snapshot of ${formatDate(inventory.recordedAt)}`
		: "Rebuilt from today's balances"
}

function Summary({ summary }: { summary: ProfitabilityDto }) {
	const { totals, series, bucket, books, startInventory } = summary
	const fills = totals.buys + totals.sells
	const periodLabel = PERIODS.find((entry) => entry.value === summary.period)?.label.toLowerCase() ?? ""

	return (
		<>
			<section className="operator-metrics" aria-label="Period summary">
				<Figure
					label="Profit"
					value={formatUsd(totals.profitUsd, { signed: true })}
					tone={toneOf(totals.profitUsd)}
					note={`${fills.toLocaleString()} ${fills === 1 ? "fill" : "fills"} in ${periodLabel === "all time" ? "all" : periodLabel}`}
				/>
				<Figure
					label="Return on inventory"
					value={formatPercent(totals.returnPct)}
					tone={toneOf(totals.returnPct)}
					note={totals.returnPct === null ? describeNoReturn(totals) : "Profit over starting inventory"}
				/>
				<Figure
					label="Starting inventory"
					value={startInventory ? formatUsd(startInventory.usd) : "—"}
					note={startInventory ? describeSource(startInventory) : "Not on record for this period"}
				/>
				<Figure label="Volume" value={formatUsd(totals.boughtUsd + totals.soldUsd)} note="Bought and sold" />
			</section>

			{startInventory && startInventory.tokens.length > 0 ? (
				<p className="analytics-holdings">The period began with {describeHoldings(startInventory.tokens)}.</p>
			) : null}

			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Profit</span>
						<h2>Running total and each {BUCKET_NOUNS[bucket].one}</h2>
					</div>
				</div>
				{fills === 0 ? (
					<p className="operator-empty">Nothing filled in this period.</p>
				) : (
					<ProfitCharts series={series} bucket={bucket} />
				)}
			</section>

			{books.length > 0 ? (
				<section className="operator-section">
					<div className="operator-section-heading">
						<div>
							<span className="eyebrow">By pair</span>
							<h2>Buys against sells</h2>
						</div>
					</div>
					<div className="analytics-table-frame">
						<table className="analytics-table">
							<thead>
								<tr>
									<th scope="col">Pair</th>
									<th scope="col">Bought</th>
									<th scope="col">Average buy</th>
									<th scope="col">Sold</th>
									<th scope="col">Average sell</th>
									<th scope="col">Spread</th>
									<th scope="col">Inventory change</th>
									<th scope="col">Profit</th>
								</tr>
							</thead>
							<tbody>
								{books.map((book) => (
									<BookRow key={book.book} book={book} />
								))}
							</tbody>
						</table>
					</div>
				</section>
			) : null}

			{fills > 0 ? <Buckets key={summary.period} summary={summary} /> : null}

			<p className="analytics-method">
				Profit is the change your buys and sells made to what you hold: what they took in, less what they paid
				out, with each token valued at its latest rate. Volume bought and not yet sold counts at that rate, so
				the figure moves with it. The return compares that profit with the inventory you held when the period
				began, taken from a daily snapshot or, for the last seven days, rebuilt from today's balances. Network
				fees are not included.
				{summary.estimatedFills > 0
					? ` ${summary.estimatedFills.toLocaleString()} ${summary.estimatedFills === 1 ? "fill in this period was" : "fills in this period were"} recorded before fills kept what they took in, and ${summary.estimatedFills === 1 ? "is" : "are"} priced at the order's own rate.`
					: ""}
				{summary.unpricedTokens.length > 0
					? ` ${summary.unpricedTokens.join(", ")} ${summary.unpricedTokens.length === 1 ? "has" : "have"} no dollar price, so the dollar figures leave ${summary.unpricedTokens.length === 1 ? "it" : "them"} out.`
					: ""}
			</p>
		</>
	)
}

function Figure(props: { label: string; value: string; note: string; tone?: string }) {
	return (
		<div>
			<span>{props.label}</span>
			<strong data-tone={props.tone}>{props.value}</strong>
			<small>{props.note}</small>
		</div>
	)
}

function BookRow({ book }: { book: BookProfit }) {
	return (
		<tr>
			<td>
				<span className="analytics-pair">
					<TokenPairIcons tokenA={book.base} tokenB={book.quote} />
					<strong>
						{book.base} / {book.quote}
					</strong>
				</span>
			</td>
			<td>
				{formatAmount(book.bought)} <small>{book.base}</small>
			</td>
			<td>{book.averageBuy === null ? "—" : formatRate(book.averageBuy)}</td>
			<td>
				{formatAmount(book.sold)} <small>{book.base}</small>
			</td>
			<td>{book.averageSell === null ? "—" : formatRate(book.averageSell)}</td>
			<td>{formatPercent(book.spreadPct)}</td>
			<td>
				<span className="analytics-change">
					<span>{formatChange(book.baseChange, book.base)}</span>
					<span>{formatChange(book.quoteChange, book.quote)}</span>
				</span>
			</td>
			<td>
				{book.profitUsd === null ? (
					"—"
				) : (
					<strong data-tone={toneOf(book.profitUsd)}>{formatUsd(book.profitUsd, { signed: true })}</strong>
				)}
			</td>
		</tr>
	)
}

/** Every bucket in the period as a row, latest first: the charts' figures, to read rather than to look at. */
function Buckets({ summary }: { summary: ProfitabilityDto }) {
	const [page, setPage] = useState(1)
	const { bucket } = summary
	const noun = BUCKET_NOUNS[bucket]
	const rows = [...summary.series].reverse()
	const current = Math.min(page, Math.max(1, Math.ceil(rows.length / PAGE_SIZE)))
	const shown = rows.slice((current - 1) * PAGE_SIZE, current * PAGE_SIZE)
	const traded = (figures: ProfitFigures) => figures.buys + figures.sells > 0

	return (
		<section className="operator-section">
			<div className="operator-section-heading">
				<div>
					<span className="eyebrow">By {noun.one}</span>
					<h2>Every {noun.one} in the period</h2>
				</div>
			</div>
			<div className="analytics-table-frame">
				<table className="analytics-table">
					<thead>
						<tr>
							<th scope="col">{noun.one}</th>
							<th scope="col">Bought</th>
							<th scope="col">Sold</th>
							<th scope="col">Starting inventory</th>
							<th scope="col">Return</th>
							<th scope="col">Profit</th>
						</tr>
					</thead>
					<tbody>
						{shown.map((row) => (
							<tr key={row.start} data-quiet={traded(row) ? undefined : true}>
								<td>{bucketLabel(row.start, bucket)}</td>
								<td>{traded(row) ? formatUsd(row.boughtUsd) : "—"}</td>
								<td>{traded(row) ? formatUsd(row.soldUsd) : "—"}</td>
								<td>{row.startInventoryUsd === null ? "—" : formatUsd(row.startInventoryUsd)}</td>
								<td>{traded(row) ? formatPercent(row.returnPct) : "—"}</td>
								<td>
									{traded(row) ? (
										<strong data-tone={toneOf(row.profitUsd)}>
											{formatUsd(row.profitUsd, { signed: true })}
										</strong>
									) : (
										"—"
									)}
								</td>
							</tr>
						))}
					</tbody>
				</table>
			</div>
			{rows.length > PAGE_SIZE ? (
				<Pager page={current} pageSize={PAGE_SIZE} total={rows.length} noun={noun.many} onPage={setPage} />
			) : null}
		</section>
	)
}
