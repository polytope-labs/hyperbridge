import { CurveEditor } from "../../components/CurveEditor"
import { TokenIcon, TokenPairIcons } from "../../components/TokenIcon"
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
	const hasBid = Boolean(strategy.bid || draft.enableBid)
	const hasAsk = Boolean(strategy.ask || draft.enableAsk)
	const directionLabel = hasBid && hasAsk ? "Two-way market" : hasBid ? "Buy side only" : "Sell side only"

	if (strategy.pricingMode === "venue") {
		return (
			<section className="market-editor operator-market-editor">
				<div className="operator-market-summary operator-market-summary-venue">
					<div>
						<span className="markets-kicker">Uniswap V4 pricing</span>
						<h3>{editor.title}</h3>
						<p>Rates follow the configured on-chain liquidity position.</p>
					</div>
				</div>
			</section>
		)
	}

	return (
		<section className="market-editor operator-market-editor">
			<div className="operator-market-summary">
				<TokenPairIcons tokenA={token0} tokenB={token1} />
				<div>
					<span className="markets-kicker">
						{strategy.referenceOnly ? "Reference pricing" : "Manual pricing"}
					</span>
					<h3>{token1} per {token0}</h3>
					<p>
						{strategy.sameToken
							? "Keep every return below 1; the difference is the spread earned on each fill."
							: strategy.referenceOnly
								? "This rate anchors confirmation sizing and does not fill customer orders."
								: `The buy rate is ${token1} received by Simplex; the sell rate is ${token1} paid out. Keep buy above sell to preserve your spread.`}
					</p>
				</div>
				<span className="operator-market-side-status">{directionLabel}</span>
			</div>
			{editor.crossedAt !== null ? (
				<div className="operator-market-warning" role="alert">
					<strong>Spread crosses at order size {editor.crossedAt}</strong>
					<span>The buy rate is at or below the sell rate, so a round trip at this size loses money.</span>
				</div>
			) : null}
			{!strategy.referenceOnly ? (
				<section className="operator-market-panel operator-market-cap-panel" aria-labelledby="order-limit-title">
					<header className="operator-market-panel-heading">
						<div>
							<span className="markets-kicker">Risk limit</span>
							<h3 id="order-limit-title">Maximum order size</h3>
						</div>
						<span className="operator-market-panel-value">
							{editor.capCleared ? "No limit" : `${draft.maxOrderSize} ${token0}`}
						</span>
					</header>
					<p className="operator-market-panel-description">
						Orders above this {token0} amount are skipped. Leave the field empty to accept orders of any size.
					</p>
					<div className="operator-market-cap-controls">
						<label className="field market-limit-field">
							<span>Amount in {token0} <small>Optional</small></span>
							<input
								type="text"
								value={draft.maxOrderSize}
								placeholder="No limit"
								onChange={(event) => editor.patch({ maxOrderSize: event.target.value })}
							/>
						</label>
						<button
							type="button"
							className="secondary"
							disabled={!editor.capChanged || status.busy}
							onClick={editor.applyMaxOrderSize}
						>
							{editor.capCleared ? "Remove limit" : "Save limit"}
						</button>
					</div>
				</section>
			) : null}
			<section className="operator-market-panel operator-market-pricing-panel" aria-labelledby="pricing-title">
				<header className="operator-market-panel-heading">
					<div>
						<span className="markets-kicker">Price settings</span>
						<h3 id="pricing-title">Set a price for each direction</h3>
					</div>
					<span className="operator-market-panel-value">{token1} per {token0}</span>
				</header>
				<div className="market-curve-grid">
					{hasBid ? (
						<section className="market-curve">
							<header className="market-curve-toggle">
								<span className="operator-market-direction-icon">
									<TokenIcon symbol={token1} size="md" />
								</span>
								<span className="operator-market-direction-copy">
									<small>Buy side</small>
									<strong>Simplex buys {token1}</strong>
									<p>
										Customers send {token1} and receive {token0}.
									</p>
								</span>
							</header>
							<CurveEditor
								points={draft.bid}
								onChange={(bid) => editor.patch({ bid })}
								amountLabel={`Order size (${token0})`}
								valueLabel={`${token1} received per ${token0}`}
								minPoints={strategy.sameToken || strategy.referenceOnly ? 1 : 0}
							/>
						</section>
					) : null}
					{hasAsk ? (
						<section className="market-curve">
							<header className="market-curve-toggle">
								<span className="operator-market-direction-icon">
									<TokenIcon symbol={token1} size="md" />
								</span>
								<span className="operator-market-direction-copy">
									<small>{strategy.sameToken ? "Delivery side" : "Sell side"}</small>
									<strong>
										{strategy.sameToken ? `Deliver ${token0}` : `Simplex sells ${token1}`}
									</strong>
									<p>
										{strategy.sameToken
											? "Keep the return below 1; the difference is your spread."
											: `Customers send ${token0} and receive ${token1}.`}
									</p>
								</span>
							</header>
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
				{!strategy.sameToken && !strategy.referenceOnly && !hasBid ? (
					<div className="operator-market-add-side">
						<div>
							<strong>Buy side is off</strong>
							<span>Add a buy curve to quote this market in both directions.</span>
						</div>
						<button
							type="button"
							className="secondary"
							onClick={() => editor.patch({ bid: [{ amount: "1", value: "" }], enableBid: true })}
						>
							Add buy side
						</button>
					</div>
				) : null}
				{!strategy.sameToken && !strategy.referenceOnly && !hasAsk ? (
					<div className="operator-market-add-side">
						<div>
							<strong>Sell side is off</strong>
							<span>Add a sell curve to quote this market in both directions.</span>
						</div>
						<button
							type="button"
							className="secondary"
							onClick={() => editor.patch({ ask: [{ amount: "1", value: "" }], enableAsk: true })}
						>
							Add sell side
						</button>
					</div>
				) : null}
			</section>
			<div className="operator-market-actions">
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
				<div className="operator-market-action-status" aria-live="polite">
					{status.message ? <span className="badge ok">{status.message}</span> : null}
					{status.error ? <span className="badge err">{status.error}</span> : null}
				</div>
				<button type="button" className="primary" onClick={editor.applyCurves} disabled={status.busy}>
					{status.busy ? "Applying…" : "Apply price changes"}
				</button>
			</div>
		</section>
	)
}
