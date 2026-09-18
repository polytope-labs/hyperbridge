import { useStrategyEditor } from "./useStrategyEditor"
import type { AdminStrategyDto } from "../../types"

/**
 * One live market: what it trades, and the option to close it.
 *
 * There is nothing to price here. A market says the pair is open; what the
 * filler will pay on it comes from the limit orders posted against it.
 */
export function StrategyMarketEditor(props: {
	strategy: AdminStrategyDto
	onApplied: () => Promise<void> | void
	removable: boolean
	hasVaults: boolean
}) {
	const editor = useStrategyEditor({
		strategy: props.strategy,
		onApplied: props.onApplied,
		hasVaults: props.hasVaults,
	})

	return (
		<section className="market-editor operator-market-editor">
			<div className="operator-market-summary">
				<h3>{editor.title}</h3>
				<p className="hint">
					{props.strategy.sameToken
						? `Same-asset transfers of ${editor.token0} between chains.`
						: `Swaps between ${editor.token0} and ${editor.token1}.`}{" "}
					Post a limit order to set what this market pays.
				</p>
			</div>

			{editor.status.error ? <p className="error">{editor.status.error}</p> : null}

			<div className="operator-market-actions">
				<button
					type="button"
					className="danger"
					onClick={editor.removeMarket}
					disabled={editor.status.busy || !props.removable}
					title={props.removable ? undefined : "The last market cannot be removed"}
				>
					Remove market
				</button>
			</div>
		</section>
	)
}
