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
	const { draft, patch, resolved0 } = market

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
					<p className="hint">
						Simplex can buy or sell either side of this pair. Post a limit order to set what it pays.
					</p>
				</div>
			</div>
			<div className="market-asset-grid">
				{symbolSelect(draft.token0, "First asset", "token0")}
				{symbolSelect(draft.token1, "Second asset", "token1")}
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
