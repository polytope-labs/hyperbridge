import { useState } from "react"
import { isRegistrySymbol } from "@/config/asset-registry"
import { TokenPairIcons } from "../../components/TokenIcon"
import { WizardDialog } from "../../components/WizardDialog"
import { newCrossAssetDraft, normSymbol, removeAt } from "../state"
import { MarketRow } from "../strategies/MarketRow"
import { useStrategiesModel } from "../strategies/useStrategiesModel"
import type { StepProps } from "../Wizard"


export function StepStrategies({ state, setState, defaults }: StepProps) {
	const [editingPairIndex, setEditingPairIndex] = useState<number | null>(null)
	const model = useStrategiesModel({ state, setState, defaults })
	const { chains, availableSymbols, marketRows, enabled, duplicateKeys, defaultToken1, patchPair } = model

	const editingPair = editingPairIndex === null ? null : state.pairs[editingPairIndex]

	return (
		<div className="wizard-sections strategies-step">
			<section className="card market-flow-section markets-primary-section">
				<div className="market-flow-heading markets-section-heading">
					<div>
						<span className="market-flow-step">2 · Trading markets</span>
						<h2>Which token pairs should Simplex fill?</h2>
						<p className="hint">Each market can be reviewed and priced in a focused editor.</p>
					</div>
					<button
						type="button"
						className="market-create-button"
						onClick={() => {
							const index = state.pairs.length
							setState((s) => ({ ...s, pairs: [...s.pairs, newCrossAssetDraft(defaultToken1)] }))
							setEditingPairIndex(index)
						}}
					>
						<span aria-hidden="true">+</span> Create market
					</button>
				</div>

				<div className="market-overview-list">
					{marketRows.map(({ pair, index }) => (
						<div className="market-overview-row" key={index}>
							<TokenPairIcons tokenA={pair.token0} tokenB={pair.token1} />
							<div className="market-overview-copy">
								<strong>
									{pair.token0 || "Choose asset"} <span>↔</span> {pair.token1 || "Choose asset"}
								</strong>
							</div>
							<button
								type="button"
								className="market-configure-button"
								onClick={() => setEditingPairIndex(index)}
							>
								Configure
							</button>
						</div>
					))}
				</div>
			</section>

			<WizardDialog
				open={editingPair !== null}
				onClose={() => setEditingPairIndex(null)}
				title={
					editingPair
						? `${editingPair.token0 || "New"} ↔ ${editingPair.token1 || "market"}`
						: "Configure market"
				}
				description="Choose the token pair, order limit, and how each direction should be priced."
			>
				{editingPair && editingPairIndex !== null && (
					<>
						<MarketRow
							pair={editingPair}
							symbols={availableSymbols}
							usdStables={defaults.usdStables}
							chains={chains}
							duplicate={duplicateKeys.has(
								`${normSymbol(editingPair.token0)}/${normSymbol(editingPair.token1)}`,
							)}
							customAssets={state.customAssets}
							onPatch={(patch) => patchPair(editingPairIndex, patch)}
							onSymbolChange={(patch) => patchPair(editingPairIndex, patch)}
							onRenameAsset={(from, to) =>
								setState((s) => {
									const customAssets = { ...s.customAssets }
									const stillUsed = s.pairs.some(
										(p, i) => i !== editingPairIndex && (p.token0 === from || p.token1 === from),
									)
									if (from && customAssets[from] && from !== to && !stillUsed) {
										if (to && !isRegistrySymbol(to)) customAssets[to] = customAssets[from]
										delete customAssets[from]
									}
									return { ...s, customAssets }
								})
							}
							onCustomAddress={(symbol, chain, address) =>
								setState((s) =>
									isRegistrySymbol(symbol)
										? s
										: {
												...s,
												customAssets: {
													...s.customAssets,
													[symbol]: { ...(s.customAssets[symbol] ?? {}), [chain]: address },
												},
											},
								)
							}
						/>
						<footer className="market-dialog-footer">
							<button
								type="button"
								className="market-delete-button"
								onClick={() => {
									setState((s) => ({ ...s, pairs: removeAt(s.pairs, editingPairIndex) }))
									setEditingPairIndex(null)
								}}
							>
								Delete market
							</button>
							<button type="button" className="primary" onClick={() => setEditingPairIndex(null)}>
								Done
							</button>
						</footer>
					</>
				)}
			</WizardDialog>
		</div>
	)
}
