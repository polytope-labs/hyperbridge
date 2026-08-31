import { CurveEditor } from "../../components/CurveEditor"
import type { AdminStrategyDto } from "../../types"
import { useStrategyEditor } from "./useStrategyEditor"

export function StrategyMarketEditor(props: {
	strategy: AdminStrategyDto
	onApplied: () => Promise<void> | void
	removable: boolean
	hasVaults: boolean
}) {
	const { strategy, onApplied, removable, hasVaults } = props
	const editor = useStrategyEditor({ strategy, onApplied, hasVaults })
	const { draft, status, token0, token1 } = editor

	if (strategy.pricingMode === "venue") {
		return (
			<section className="market-editor operator-market-editor">
				<div className="market-editor-heading">
					<div>
						<span className="markets-kicker">Uniswap V4 pricing</span>
						<h3>{editor.title}</h3>
					</div>
				</div>
				<p className="market-editor-note">
					Prices come from the configured on-chain position and cannot be edited here.
				</p>
			</section>
		)
	}

	return (
		<section className="market-editor operator-market-editor">
			<div className="market-editor-heading">
				<div>
					<span className="markets-kicker">Live market</span>
					<h3>{editor.title}</h3>
				</div>
			</div>
			{strategy.sameToken ? (
				<p className="hint">
					Ask-only: the price is the fraction of the input paid back out. Keep every point strictly below 1 —
					the gap to 1 is the spread on each fill.
				</p>
			) : null}
			{strategy.referenceOnly ? (
				<p className="hint">
					Price feed only: edits update the USD anchor rate for confirmation sizing. This market never fills
					orders.
				</p>
			) : null}
			{!strategy.sameToken && !strategy.referenceOnly ? (
				<p className="hint">
					Prices are {token1} per {token0}: the bid is the {token1} you receive per {token0} paid out when
					buying, the ask is the {token1} you pay out per {token0} received when selling. Keep the bid above
					the ask everywhere — the gap is your spread. Delete every point on a side and Apply to close that
					direction (one-sided LP).
				</p>
			) : null}
			{editor.crossedAt !== null ? (
				<p className="hint">
					⚠ The book is crossed at order size {editor.crossedAt} (bid at or below ask) — both sides still
					fill at their own curve, but a full round trip at these prices loses money. Leave it only if
					deliberate.
				</p>
			) : null}
			{!strategy.referenceOnly ? (
				<div className="market-asset-grid operator-market-cap-editor">
					<label className="field market-limit-field">
						<span>
							Maximum order in {token0} <em>Optional</em>
						</span>
						<input
							type="text"
							value={draft.maxOrderSize}
							placeholder="uncapped"
							onChange={(event) => editor.patch({ maxOrderSize: event.target.value })}
						/>
					</label>
					<button
						type="button"
						disabled={!editor.capChanged || status.busy}
						onClick={editor.applyMaxOrderSize}
					>
						{editor.capCleared ? "Remove cap" : "Save cap"}
					</button>
					<span className="market-editor-note">
						Per-order cap on the {token0} notional — orders needing more than the cap allows are skipped.
						Leave it empty to remove the cap and fill every order at its full notional. Either change binds
						from the next order.
					</span>
				</div>
			) : null}
			<div className="market-curves">
				<div className="market-pricing-heading">
					<div>
						<span className="markets-kicker">Price settings</span>
						<h3>Set a price for each direction</h3>
					</div>
					<p className="market-editor-note">
						Rates are shown as {token1} per {token0}.
					</p>
				</div>
				<div className="market-curve-grid">
					{strategy.bid || draft.enableBid ? (
						<section className="market-curve">
							<div className="market-curve-toggle">
								<span>
									<strong>Simplex buys {token1}</strong>
									<small>
										Customers send {token1} and receive {token0}.
									</small>
								</span>
							</div>
							<CurveEditor
								points={draft.bid}
								onChange={(bid) => editor.patch({ bid })}
								amountLabel={`Order size (${token0})`}
								valueLabel={`${token1} received per ${token0}`}
								minPoints={strategy.sameToken || strategy.referenceOnly ? 1 : 0}
							/>
						</section>
					) : null}
					{strategy.ask || draft.enableAsk ? (
						<section className="market-curve">
							<div className="market-curve-toggle">
								<span>
									<strong>
										{strategy.sameToken ? `Deliver ${token0}` : `Simplex sells ${token1}`}
									</strong>
									<small>
										{strategy.sameToken
											? "Keep the return below 1; the difference is your spread."
											: `Customers send ${token0} and receive ${token1}.`}
									</small>
								</span>
							</div>
							<CurveEditor
								points={draft.ask}
								onChange={(ask) => editor.patch({ ask })}
								amountLabel={`Order size (${token0})`}
								valueLabel={strategy.sameToken ? "Price (below 1)" : `${token1} paid per ${token0}`}
								minPoints={strategy.sameToken || strategy.referenceOnly ? 1 : 0}
							/>
						</section>
					) : null}
				</div>
			</div>
			{!strategy.sameToken && !strategy.referenceOnly && !strategy.bid && !draft.enableBid ? (
				<button
					type="button"
					onClick={() => editor.patch({ bid: [{ amount: "0", value: "" }], enableBid: true })}
				>
					Enable bid side (one-sided LP → both directions)
				</button>
			) : null}
			{!strategy.sameToken && !strategy.referenceOnly && !strategy.ask && !draft.enableAsk ? (
				<button
					type="button"
					onClick={() => editor.patch({ ask: [{ amount: "0", value: "" }], enableAsk: true })}
				>
					Enable ask side (one-sided LP → both directions)
				</button>
			) : null}
			<div className="operator-market-actions">
				<button type="button" className="primary" onClick={editor.applyCurves} disabled={status.busy}>
					{status.busy ? "Applying…" : "Apply"}
				</button>
				{removable ? (
					<button
						type="button"
						className="operator-market-delete"
						onClick={editor.removeMarket}
						disabled={status.busy}
					>
						Stop and delete market
					</button>
				) : null}
				{status.message ? <span className="badge ok">{status.message}</span> : null}
				{status.error ? <span className="badge err">{status.error}</span> : null}
			</div>
		</section>
	)
}
