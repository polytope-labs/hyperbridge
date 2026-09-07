import { CurveEditor } from "../../components/CurveEditor"
import { TokenSelect } from "../../components/TokenSelect"
import { CUSTOM_TOKEN } from "./marketModel"
import { useCreateMarket } from "./useCreateMarket"

export function CreateMarketForm(props: {
	symbols: string[]
	chains: number[]
	chainLabel: (id: number | string) => string
	onAdded: () => Promise<void> | void
	onCancel: () => void
}) {
	const { symbols, chains, chainLabel, onAdded, onCancel } = props
	const market = useCreateMarket({ symbols, onAdded })
	const { draft, patch, resolved0, resolved1 } = market

	const symbolSelect = (value: string, label: string, field: "token0" | "token1") => (
		<div className="field market-token-field">
			<span className="field-label field-label-required-mark">
				{label}
				<span className="field-required-mark" aria-hidden="true">
					*
				</span>
			</span>
			<TokenSelect
				label={label}
				value={value === CUSTOM_TOKEN ? draft.customSymbol : value}
				symbols={symbols}
				custom={value === CUSTOM_TOKEN}
				onSelect={(token) => patch({ [field]: token })}
				onCustom={() => patch({ [field]: CUSTOM_TOKEN })}
			/>
		</div>
	)

	return (
		<section className="market-editor operator-market-editor">
			<div className="market-editor-heading">
				<div>
					<span className="markets-kicker">Token pair</span>
					<h3>Choose the two assets</h3>
					<p className="hint">Simplex can buy or sell either side of this pair.</p>
				</div>
			</div>
			<div className="market-asset-grid">
				{symbolSelect(draft.token0, "First asset", "token0")}
				{symbolSelect(draft.token1, "Second asset", "token1")}
				<label className="field market-limit-field">
					<span>Maximum order in {resolved0 || "the first asset"} [Optional]</span>
					<input
						type="text"
						value={draft.maxOrderSize}
						onChange={(event) => patch({ maxOrderSize: event.target.value })}
					/>
				</label>
			</div>
			{market.customSide ? (
				<div className="market-custom-assets">
					<label className="field">
						<span>Custom token symbol</span>
						<input
							type="text"
							placeholder="e.g. BRZ"
							value={draft.customSymbol}
							onChange={(event) => patch({ customSymbol: event.target.value })}
						/>
					</label>
					{chains.map((id) => {
						const chainKey = `EVM-${id}`
						return (
							<div className="custom-asset-chain" key={id}>
								<span>{chainLabel(id)}</span>
								<div className="custom-asset-chain-control">
									<input
										type="text"
										aria-label={`${chainLabel(id)} custom token address`}
										placeholder="0x… (leave blank if not deployed here)"
										value={draft.customAddresses[chainKey] ?? ""}
										onChange={(event) =>
											patch({
												customAddresses: {
													...draft.customAddresses,
													[chainKey]: event.target.value,
												},
											})
										}
									/>
									<button type="button" onClick={() => void market.verifyToken(chainKey)}>
										Verify
									</button>
								</div>
								{draft.verified[chainKey] ? (
									<span className="hint">{draft.verified[chainKey]}</span>
								) : null}
							</div>
						)
					})}
				</div>
			) : null}
			<div className="market-curves">
				<div className="market-pricing-heading">
					<div>
						<span className="markets-kicker">Price settings</span>
						<h3>Set a price for each direction</h3>
					</div>
					<p className="market-editor-note">
						Rates are shown as {resolved1 || "second asset"} per {resolved0 || "first asset"}.
					</p>
				</div>
				<div className="market-curve-grid">
					<MarketDirection
						enabled={draft.bidEnabled}
						onEnabledChange={(bidEnabled) => patch({ bidEnabled })}
						title={`Simplex buys ${resolved1 || "the second asset"}`}
						description={`Customers send ${resolved1 || "the second asset"} and receive ${resolved0 || "the first asset"}.`}
						points={draft.bid}
						onPointsChange={(bid) => patch({ bid })}
						amountLabel={`Order size (${resolved0 || "first asset"})`}
						valueLabel={`${resolved1 || "Second asset"} received per ${resolved0 || "first asset"}`}
					/>
					<MarketDirection
						enabled={draft.askEnabled}
						onEnabledChange={(askEnabled) => patch({ askEnabled })}
						title={`Simplex sells ${resolved1 || "the second asset"}`}
						description={`Customers send ${resolved0 || "the first asset"} and receive ${resolved1 || "the second asset"}.`}
						points={draft.ask}
						onPointsChange={(ask) => patch({ ask })}
						amountLabel={`Order size (${resolved0 || "first asset"})`}
						valueLabel={`${resolved1 || "Second asset"} paid per ${resolved0 || "first asset"}`}
					/>
				</div>
			</div>
			{market.crossedAt !== null ? (
				<p className="hint">
					⚠ The book is crossed at order size {market.crossedAt} (bid at or below ask) — both sides still
					fill at their own curve, but a full round trip at these prices loses money. Leave it only if
					deliberate.
				</p>
			) : null}
			<div className="operator-market-actions">
				<button type="button" className="primary" onClick={() => void market.submit()} disabled={market.busy}>
					{market.busy ? "Creating…" : "Create market"}
				</button>
				<button type="button" onClick={onCancel}>
					Cancel
				</button>
				{market.error ? <span className="badge err">{market.error}</span> : null}
			</div>
		</section>
	)
}

function MarketDirection(props: {
	enabled: boolean
	onEnabledChange: (enabled: boolean) => void
	title: string
	description: string
	points: Parameters<typeof CurveEditor>[0]["points"]
	onPointsChange: Parameters<typeof CurveEditor>[0]["onChange"]
	amountLabel: string
	valueLabel: string
}) {
	return (
		<section className="market-curve">
			<label className="market-curve-toggle">
				<input
					type="checkbox"
					checked={props.enabled}
					onChange={(event) => props.onEnabledChange(event.target.checked)}
				/>
				<span>
					<strong>{props.title}</strong>
					<small>{props.description}</small>
				</span>
			</label>
			{props.enabled ? (
				<CurveEditor
					points={props.points}
					onChange={props.onPointsChange}
					amountLabel={props.amountLabel}
					valueLabel={props.valueLabel}
				/>
			) : null}
		</section>
	)
}
