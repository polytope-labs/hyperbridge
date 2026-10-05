import { useState } from "react"
import { Pager } from "../components/Pager"
import { PillTabs } from "../components/PillTabs"
import { TokenPairIcons } from "../components/TokenIcon"
import { formatAmount } from "../lib/format"
import type { BookProfit, ProfitabilityDto, ProfitFigures, ProfitPeriod } from "../types"
import {
	BUCKET_NOUNS,
	bucketLabel,
	describeNoSpread,
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
 * Profit is realized when volume bought is sold again, or volume sold is bought back, at the
 * average cost of what was held. So a period's figures are its fills closing against everything
 * before them, and what is still held shows as an open position rather than as profit or loss.
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

function Summary({ summary }: { summary: ProfitabilityDto }) {
	const { totals, series, bucket, books } = summary
	const fills = totals.buys + totals.sells
	const periodLabel = PERIODS.find((entry) => entry.value === summary.period)?.label.toLowerCase() ?? ""

	return (
		<>
			<section className="operator-metrics" aria-label="Period summary">
				<Figure
					label="Realized profit"
					value={formatUsd(totals.realizedUsd, { signed: true })}
					tone={toneOf(totals.realizedUsd)}
					note={`${fills.toLocaleString()} ${fills === 1 ? "fill" : "fills"} in ${periodLabel === "all time" ? "all" : periodLabel}`}
				/>
				<Figure
					label="Spread"
					value={formatPercent(totals.spreadPct)}
					tone={toneOf(totals.spreadPct)}
					note={totals.spreadPct === null ? describeNoSpread(totals) : "Profit over matched volume"}
				/>
				<Figure label="Matched volume" value={formatUsd(totals.matchedUsd)} note="Bought and sold again" />
				<Figure label="Open position" value={formatUsd(summary.openPositionUsd)} note="Not yet matched" />
			</section>

			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Realized profit</span>
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
									<th scope="col">Realized profit</th>
									<th scope="col">Open position</th>
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
				Profit is realized when volume bought is sold again, or volume sold is bought back, at the average cost
				of what you held. It is counted in the pair's quote token and shown in dollars at the rate of the fill
				that realized it. An open position is valued at cost, not at today's rate. Network fees are not
				included.
				{summary.estimatedFills > 0
					? ` ${summary.estimatedFills.toLocaleString()} ${summary.estimatedFills === 1 ? "fill in this period was" : "fills in this period were"} recorded before fills kept what they took in, and ${summary.estimatedFills === 1 ? "is" : "are"} priced at the order's own rate.`
					: ""}
				{summary.unpricedBooks.length > 0
					? ` ${summary.unpricedBooks.join(", ")} ${summary.unpricedBooks.length === 1 ? "has" : "have"} no dollar-stable token, so the dollar figures leave ${summary.unpricedBooks.length === 1 ? "it" : "them"} out.`
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

/** Base held open, signed: bought and not yet sold reads as a plus. */
function describePosition(book: BookProfit): string {
	if (book.position === 0) return "—"
	return `${book.position > 0 ? "+" : "-"}${formatAmount(Math.abs(book.position))} ${book.base}`
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
				{/* A pair with no dollar-stable token has its profit only in its own quote. */}
				<strong data-tone={toneOf(book.realizedUsd ?? book.realized)}>
					{book.realizedUsd === null
						? `${formatAmount(book.realized)} ${book.quote}`
						: formatUsd(book.realizedUsd, { signed: true })}
				</strong>
			</td>
			<td>{describePosition(book)}</td>
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
							<th scope="col">Matched</th>
							<th scope="col">Spread</th>
							<th scope="col">Realized profit</th>
						</tr>
					</thead>
					<tbody>
						{shown.map((row) => (
							<tr key={row.start} data-quiet={traded(row) ? undefined : true}>
								<td>{bucketLabel(row.start, bucket)}</td>
								<td>{traded(row) ? formatUsd(row.boughtUsd) : "—"}</td>
								<td>{traded(row) ? formatUsd(row.soldUsd) : "—"}</td>
								<td>{traded(row) ? formatUsd(row.matchedUsd) : "—"}</td>
								<td>{formatPercent(row.spreadPct)}</td>
								<td>
									{traded(row) ? (
										<strong data-tone={toneOf(row.realizedUsd)}>
											{formatUsd(row.realizedUsd, { signed: true })}
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
