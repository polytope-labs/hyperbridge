import type { MouseEvent } from "react"
import { TAB_PATHS } from "../../lib/route"
import { describeNoReturn, formatPercent, formatUsd, toneOf } from "./analyticsModel"
import { useProfitability } from "./useProfitability"
import "../../styles/analytics.css"

/**
 * The overview's one line on whether the last seven days made money: what the week's buys and
 * sells added to inventory, as a share of the inventory the week began with. Green when it grew
 * and red when it shrank. It opens the analytics page, which is where the figure is taken apart.
 */
export function ReturnMetric(props: { onOpen: () => void }) {
	const { summary } = useProfitability("7d")
	const totals = summary?.totals
	const change = totals?.returnPct ?? null

	// A real link, so it can be opened in a new tab; a plain click stays in the app.
	const open = (event: MouseEvent<HTMLAnchorElement>) => {
		if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return
		event.preventDefault()
		props.onOpen()
	}

	return (
		<a className="operator-metric-link" href={TAB_PATHS.analytics} onClick={open}>
			<span>7-day return</span>
			<strong data-tone={toneOf(change)}>{formatPercent(change)}</strong>
			{totals ? (
				<small>
					{totals.returnPct !== null && totals.startInventoryUsd !== null
						? `${formatUsd(totals.profitUsd, { signed: true })} on ${formatUsd(totals.startInventoryUsd)}`
						: describeNoReturn(totals)}
				</small>
			) : null}
		</a>
	)
}
