import { type RefObject, useLayoutEffect, useRef, useState } from "react"
import type { ProfitBucket, ProfitBucketFigures } from "../../types"
import {
	BUCKET_NOUNS,
	bucketLabel,
	formatUsd,
	formatUsdShort,
	labelledBuckets,
	niceTicks,
	runningTotal,
	toneOf,
} from "./analyticsModel"

/** Room for the axis figures on the left, and so the last mark is not hard against the edge. */
const LEFT = 52
const RIGHT = 12
/** A bar is a mark, not a block: capped however wide its slot. */
const BAR_MAX = 24
/** About what one axis label needs, which decides how many buckets get one. */
const LABEL_WIDTH = 64

/** The element's width, kept current, so a chart is drawn at its real size and its type stays legible. */
function useElementWidth(ref: RefObject<HTMLElement | null>, fallback: number): number {
	const [width, setWidth] = useState(fallback)
	useLayoutEffect(() => {
		const element = ref.current
		if (!element) return
		const measure = () => {
			const next = element.getBoundingClientRect().width
			if (next > 0) setWidth(next)
		}
		measure()
		if (typeof ResizeObserver === "undefined") return
		const observer = new ResizeObserver(measure)
		observer.observe(element)
		return () => observer.disconnect()
	}, [ref])
	return width
}

/** `value` on a vertical scale running from `top` (the highest tick) down to `bottom` (the lowest). */
function scaleY(ticks: number[], top: number, bottom: number): (value: number) => number {
	const low = ticks[0]
	const high = ticks[ticks.length - 1]
	return (value) => top + ((high - value) / (high - low)) * (bottom - top)
}

/**
 * Profit over the period, twice: the running total as a line, and each bucket's own
 * figure as a bar, green above the line for a gain and red below it for a loss. Two charts on one
 * time axis rather than one chart with two scales.
 *
 * Pointing at a bucket in either reads both figures out above the charts.
 */
export function ProfitCharts(props: { series: ProfitBucketFigures[]; bucket: ProfitBucket }) {
	const { series, bucket } = props
	const frame = useRef<HTMLDivElement>(null)
	const width = useElementWidth(frame, 720)
	const [pointed, setPointed] = useState<number>()

	const count = series.length
	// A period always has at least one bucket; without one there is nothing to draw an axis for.
	if (count === 0) return null
	const totals = runningTotal(series)
	const plot = Math.max(width - LEFT - RIGHT, 1)
	const slot = plot / Math.max(count, 1)
	const x = (index: number) => LEFT + slot * (index + 0.5)
	const shown = pointed !== undefined && pointed < count ? pointed : count - 1
	const labelled = labelledBuckets(count, Math.floor(plot / LABEL_WIDTH))
	const noun = BUCKET_NOUNS[bucket].one

	// The running total.
	const totalTicks = niceTicks(Math.min(...totals), Math.max(...totals))
	const totalY = scaleY(totalTicks, 16, 188)
	const line = totals.map(
		(total, index) => `${index === 0 ? "M" : "L"}${x(index).toFixed(1)},${totalY(total).toFixed(1)}`,
	)
	const area = `${line.join(" ")} L${x(count - 1).toFixed(1)},${totalY(0).toFixed(1)} L${x(0).toFixed(1)},${totalY(0).toFixed(1)} Z`

	// Each bucket.
	const values = series.map((entry) => entry.profitUsd)
	const barTicks = niceTicks(Math.min(...values), Math.max(...values))
	const barY = scaleY(barTicks, 12, 152)
	const barWidth = Math.max(2, Math.min(BAR_MAX, slot - 2))

	const readout = `${bucketLabel(series[shown].start, bucket)}: ${formatUsd(series[shown].profitUsd, { signed: true })}, ${formatUsd(totals[shown], { signed: true })} to date`

	/** One invisible column per bucket, the full height of a chart, so a thin bar is still easy to point at. */
	const columns = (height: number) =>
		series.map((entry, index) => (
			<rect
				key={entry.start}
				className="analytics-hit"
				x={LEFT + slot * index}
				y={0}
				width={slot}
				height={height}
				onMouseEnter={() => setPointed(index)}
			/>
		))

	return (
		<div className="analytics-charts" ref={frame} onMouseLeave={() => setPointed(undefined)}>
			<p className="analytics-readout" aria-live="off">
				{readout}
			</p>

			<figure>
				<figcaption>Running total</figcaption>
				<svg
					width={width}
					height={200}
					role="img"
					aria-label={`Running total of profit, ending at ${formatUsd(totals[count - 1], { signed: true })}`}
				>
					{totalTicks.map((tick) => (
						<g key={tick}>
							<line
								className="analytics-rule"
								data-zero={tick === 0 || undefined}
								x1={LEFT}
								x2={width - RIGHT}
								y1={totalY(tick)}
								y2={totalY(tick)}
							/>
							<text className="analytics-axis" x={LEFT - 8} y={totalY(tick) + 3.5} textAnchor="end">
								{formatUsdShort(tick)}
							</text>
						</g>
					))}
					<path className="analytics-area" d={area} />
					<path className="analytics-line" d={line.join(" ")} />
					{pointed !== undefined && pointed < count ? (
						<line className="analytics-cursor" x1={x(shown)} x2={x(shown)} y1={16} y2={188} />
					) : null}
					<circle className="analytics-dot" cx={x(shown)} cy={totalY(totals[shown])} r={5} />
					{columns(200)}
				</svg>
			</figure>

			<figure>
				<figcaption>
					<span>Each {noun}</span>
					<span className="analytics-key" data-tone="ok">
						Profit
					</span>
					<span className="analytics-key" data-tone="err">
						Loss
					</span>
				</figcaption>
				<svg width={width} height={180} role="img" aria-label={`Profit for each ${noun} in the period`}>
					{barTicks.map((tick) => (
						<g key={tick}>
							<line
								className="analytics-rule"
								data-zero={tick === 0 || undefined}
								x1={LEFT}
								x2={width - RIGHT}
								y1={barY(tick)}
								y2={barY(tick)}
							/>
							<text className="analytics-axis" x={LEFT - 8} y={barY(tick) + 3.5} textAnchor="end">
								{formatUsdShort(tick)}
							</text>
						</g>
					))}
					{series.map((entry, index) => {
						const value = entry.profitUsd
						if (value === 0) return null
						const zero = barY(0)
						const tip = barY(value)
						const left = x(index) - barWidth / 2
						const right = left + barWidth
						// Rounded at the end that carries the figure, square where it meets the line.
						const round = Math.min(4, barWidth / 2, Math.abs(zero - tip))
						const turn = value > 0 ? tip + round : tip - round
						const shape = `M${left},${zero} V${turn} Q${left},${tip} ${left + round},${tip} H${right - round} Q${right},${tip} ${right},${turn} V${zero} Z`
						return (
							<path
								key={entry.start}
								className="analytics-bar"
								data-tone={toneOf(value)}
								data-dim={pointed !== undefined && pointed !== index ? true : undefined}
								d={shape}
							/>
						)
					})}
					{series.map((entry, index) =>
						labelled.has(index) ? (
							<text key={entry.start} className="analytics-axis" x={x(index)} y={172} textAnchor="middle">
								{bucketLabel(entry.start, bucket, "axis")}
							</text>
						) : null,
					)}
					{columns(160)}
				</svg>
			</figure>
		</div>
	)
}
