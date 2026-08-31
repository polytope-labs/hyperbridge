import { CloseIcon } from "../../components/InterfaceIcons"
import { WizardDialog } from "../../components/WizardDialog"
import { draftHasCurve, patchAt, removeAt, type ChainDraft, type WizardState } from "../state"

export function UniswapPositionsDialog(props: {
	open: boolean
	onClose: () => void
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	chains: ChainDraft[]
}) {
	const { open, onClose, state, setState, chains } = props
	const curveMarketExists = state.pairs.some((pair) => pair.enabled && draftHasCurve(pair, state.fxPricing))
	return (
		<WizardDialog
			open={open}
			onClose={onClose}
			title="Uniswap v4 positions"
			description="Add the positions Simplex should use for live prices and fill liquidity."
		>
			<div className="uniswap-positions-modal">
				<div className="uniswap-positions-heading">
					<div>
						<h3>Positions</h3>
						<p className="hint">A price guard is optional, but both guard fields must be set together.</p>
					</div>
					<button
						type="button"
						className="market-text-action"
						onClick={() =>
							setState((current) => ({
								...current,
								fxPositions: [
									...current.fxPositions,
									{
										chain: chains[0]?.meta.stateMachineId ?? "",
										tokenId: "",
										referencePrice: "",
										maxDeviationBps: "",
									},
								],
							}))
						}
					>
						<span aria-hidden="true">+</span> Add position
					</button>
				</div>
				<div className="uniswap-position-list">
					{state.fxPositions.map((position, index) => (
						<div className="uniswap-position" key={`${position.chain}-${position.tokenId || index}`}>
							<label className="field">
								<span>Chain</span>
								<select
									value={position.chain}
									onChange={(event) =>
										setState((current) => ({
											...current,
											fxPositions: patchAt(current.fxPositions, index, {
												chain: event.target.value,
											}),
										}))
									}
								>
									{chains.map((chain) => (
										<option key={chain.meta.stateMachineId} value={chain.meta.stateMachineId}>
											{chain.meta.label}
										</option>
									))}
								</select>
							</label>
							<label className="field">
								<span className="field-label">
									Position token ID <span className="field-required">Required</span>
								</span>
								<input
									type="text"
									required
									value={position.tokenId}
									onChange={(event) =>
										setState((current) => ({
											...current,
											fxPositions: patchAt(current.fxPositions, index, {
												tokenId: event.target.value,
											}),
										}))
									}
								/>
							</label>
							<label className="field">
								<span>
									Reference price <em>Optional</em>
								</span>
								<input
									type="text"
									value={position.referencePrice}
									onChange={(event) =>
										setState((current) => ({
											...current,
											fxPositions: patchAt(current.fxPositions, index, {
												referencePrice: event.target.value,
											}),
										}))
									}
								/>
							</label>
							<label className="field">
								<span>
									Max deviation (bps) <em>Optional</em>
								</span>
								<input
									type="text"
									value={position.maxDeviationBps}
									onChange={(event) =>
										setState((current) => ({
											...current,
											fxPositions: patchAt(current.fxPositions, index, {
												maxDeviationBps: event.target.value,
											}),
										}))
									}
								/>
							</label>
							<button
								type="button"
								className="icon-button market-remove-button"
								aria-label={`Delete position ${index + 1}`}
								onClick={() =>
									setState((current) => ({
										...current,
										fxPositions: removeAt(current.fxPositions, index),
									}))
								}
							>
								<CloseIcon aria-hidden="true" />
							</button>
						</div>
					))}
				</div>
				<div className="uniswap-settings">
					<label className="field">
						<span>Fill direction</span>
						<select
							value={state.fxSide}
							onChange={(event) =>
								setState((current) => ({
									...current,
									fxSide: event.target.value as "" | "ask" | "bid",
								}))
							}
						>
							<option value="">Buy and sell</option>
							<option value="ask">Only sell the market asset</option>
							<option value="bid">Only buy the market asset</option>
						</select>
					</label>
					<label className="field">
						<span>Additional spread (bps)</span>
						<input
							type="text"
							value={state.fxSpreadBps}
							onChange={(event) =>
								setState((current) => ({ ...current, fxSpreadBps: event.target.value }))
							}
						/>
					</label>
				</div>
				{state.fxSide !== "" && curveMarketExists ? (
					<p className="error">
						A one-way fill direction requires every market to use Uniswap pricing. Disable transfer markets
						and reference feeds, or choose Buy and sell.
					</p>
				) : null}
			</div>
			<footer className="market-dialog-footer market-dialog-footer-end">
				<button type="button" className="primary" onClick={onClose}>
					Done
				</button>
			</footer>
		</WizardDialog>
	)
}
