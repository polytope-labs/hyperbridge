import type { MouseEvent } from "react"
import { TAB_PATHS } from "../../lib/route"
import { describeNoSpread, formatPercent, formatUsd, toneOf } from "./analyticsModel"
import { useProfitability } from "./useProfitability"
import "../../styles/analytics.css"

/**
 * The overview's one line on whether the last seven days made money: the spread realized on
 * volume that was both bought and sold, green when it earned and red when it lost. It opens the
 * analytics page, which is where the figure is taken apart.
 */
export function SpreadMetric(props: { onOpen: () => void }) {
	const { summary } = useProfitability("7d")
	const totals = summary?.totals
	const spread = totals?.spreadPct ?? null

	// A real link, so it can be opened in a new tab; a plain click stays in the app.
	const open = (event: MouseEvent<HTMLAnchorElement>) => {
		if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return
		event.preventDefault()
		props.onOpen()
	}

	return (
		<a className="operator-metric-link" href={TAB_PATHS.analytics} onClick={open}>
			<span>7-day spread</span>
			<strong data-tone={toneOf(spread)}>{formatPercent(spread)}</strong>
			{totals ? (
				<small>
					{spread === null
						? describeNoSpread(totals)
						: `${formatUsd(totals.realizedUsd, { signed: true })} realized`}
				</small>
			) : null}
		</a>
	)
}
