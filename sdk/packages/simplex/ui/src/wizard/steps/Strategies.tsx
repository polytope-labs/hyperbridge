import { useState } from "react"
import { isRegistrySymbol } from "@/config/asset-registry"
import { pickAnchorStable } from "@/config/pairs"
import uniswapIcon from "../../assets/networks/uniswap.svg"
import { TokenPairIcons } from "../../components/TokenIcon"
import { WizardDialog } from "../../components/WizardDialog"
import { newCrossAssetDraft, newReferenceDraft, normSymbol, removeAt } from "../state"
import { MarketRow } from "../strategies/MarketRow"
import { PricingMethodSection } from "../strategies/PricingMethodSection"
import { UniswapPositionsDialog } from "../strategies/UniswapPositionsDialog"
import { prefillCurves, useStrategiesModel } from "../strategies/useStrategiesModel"
import type { StepProps } from "../Wizard"

export function StepStrategies({ state, setState, defaults }: StepProps) {
	const [editingPairIndex, setEditingPairIndex] = useState<number | null>(null)
	const [positionsOpen, setPositionsOpen] = useState(false)
	const model = useStrategiesModel({ state, setState, defaults })
	const { chains, availableSymbols, marketRows, enabled, duplicateKeys, unanchored, defaultToken1, patchPair } = model

	const editingPair = editingPairIndex === null ? null : state.pairs[editingPairIndex]

	return (
		<div className="wizard-sections strategies-step">
			<PricingMethodSection
				value={state.fxPricing}
				onChange={(fxPricing) => setState((current) => ({ ...current, fxPricing }))}
			/>

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
								<small>
									{pair.referenceOnly
										? "Reference price only"
										: state.fxPricing === "curves"
											? "Custom price curves"
											: "Priced from Uniswap v4"}
								</small>
							</div>
							{pair.maxOrderSize.trim() && (
								<span className="market-overview-limit">
									Up to {pair.maxOrderSize} {pair.token0}
								</span>
							)}
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

				{unanchored.length > 0 && (
					<div className="market-anchor-warning">
						<p className="error">
							No USD anchor for {unanchored.join(", ")}. Add a reference-only price feed so confirmation
							depth can be sized correctly.
						</p>
						{unanchored.map((symbol) => {
							// The feed needs a stable with no existing market against the
							// symbol — a pair and its reverse are the same market.
							const stable = pickAnchorStable(enabled, symbol)
							return stable ? (
								<button
									key={symbol}
									type="button"
									className="market-text-action"
									onClick={() =>
										setState((s) => ({
											...s,
											pairs: [...s.pairs, newReferenceDraft(symbol, stable)],
										}))
									}
								>
									+ Add {stable}/{symbol} reference price feed
								</button>
							) : (
								<p className="hint" key={symbol}>
									Every USD stable already has a market against {symbol}, but none carries a curve —
									give one of those markets a price curve instead of adding a feed.
								</p>
							)
						})}
					</div>
				)}
			</section>

			{state.fxPricing === "uniswapV4" && marketRows.length > 0 && (
				<section className="card market-flow-section">
					<div className="market-flow-heading">
						<div>
							<span className="market-flow-step">3 · Liquidity source</span>
							<h2>Uniswap v4 positions</h2>
							<p className="hint">Positions provide the live price and liquidity used to fill orders.</p>
						</div>
						<button
							type="button"
							className="market-configure-button"
							onClick={() => {
								if (state.fxPositions.length === 0) {
									setState((s) => ({
										...s,
										fxPositions: [
											{
												chain: chains[0]?.meta.stateMachineId ?? "",
												tokenId: "",
												referencePrice: "",
												maxDeviationBps: "",
											},
										],
									}))
								}
								setPositionsOpen(true)
							}}
						>
							{state.fxPositions.length > 0 ? "Manage positions" : "Add position"}
						</button>
					</div>
					<div className="liquidity-summary">
						<span className="liquidity-summary-icon" aria-hidden="true">
							<img src={uniswapIcon} alt="" />
						</span>
						<div>
							<strong>
								{state.fxPositions.length} {state.fxPositions.length === 1 ? "position" : "positions"}
							</strong>
							<small>
								{state.fxPositions.length > 0
									? "Ready to price enabled markets"
									: "At least one position is required"}
							</small>
						</div>
					</div>
				</section>
			)}

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
							pricing={state.fxPricing}
							duplicate={duplicateKeys.has(
								`${normSymbol(editingPair.token0)}/${normSymbol(editingPair.token1)}`,
							)}
							customAssets={state.customAssets}
							onPatch={(patch) => patchPair(editingPairIndex, patch)}
							onSymbolChange={(patch) => {
								const next = { ...editingPair, ...patch }
								patchPair(editingPairIndex, {
									...patch,
									...prefillCurves(next, defaults.usdStables),
								})
							}}
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

			<UniswapPositionsDialog
				open={positionsOpen}
				onClose={() => setPositionsOpen(false)}
				state={state}
				setState={setState}
				chains={chains}
			/>
		</div>
	)
}
