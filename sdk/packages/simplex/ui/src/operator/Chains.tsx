import * as Collapsible from "@radix-ui/react-collapsible"
import { useState } from "react"
import { nativeTokenSymbol } from "@/cli/init/chains"
import { fundingShortfall, MIN_FEE_TOKEN_BALANCE } from "@/services/chain-funding"
import { api } from "../api"
import { ChainCollapseTrigger, isHeaderControl, useChainPanels } from "../components/ChainPanel"
import { ChainLogo } from "../components/ChainLogo"
import { EndpointVerificationStatus } from "../components/EndpointVerificationStatus"
import type { BalanceSnapshot, ChainFunding } from "../types"
import { useChainSettings } from "./chains/useChainSettings"

export function Chains(props: {
	balances: BalanceSnapshot | undefined
	/** Reloads the balances, whose `funding` says which chains can fill. */
	onBalancesChanged: () => void
}) {
	const { balances, onBalancesChanged } = props
	const model = useChainSettings()
	const panels = useChainPanels()
	const { chains, loaded, patch, saved, message, error, verifyChain, toggleChain, save } = model
	const [enabling, setEnabling] = useState<number>()
	const [fillsError, setFillsError] = useState<{ chainId: number; message: string }>()

	// Delegates the wallet on the chain now, so it fills without a restart.
	const enableFills = async (chainId: number) => {
		setEnabling(chainId)
		setFillsError(undefined)
		try {
			await api.post("/api/chains/enable-fills", { chainId })
			onBalancesChanged()
		} catch (err) {
			setFillsError({ chainId, message: err instanceof Error ? err.message : String(err) })
		} finally {
			setEnabling(undefined)
		}
	}
	// A deposit shows up on the next balance refresh; this asks for one now.
	const checkAgain = async () => {
		await api.get("/api/balances?refresh=1").catch(() => undefined)
		onBalancesChanged()
	}

	return (
		<div className="wizard-sections chains-step operator-chain-settings">
			<div className="card">
				<h2>RPC providers</h2>
				<p className="hint">
					New chains start with bundled public RPC endpoints where available. You can edit them or use your
					own providers.
				</p>
			</div>

			{chains.map((chain) => {
				// A running chain's switch enables fills there live; any other adds it at the next restart.
				const live = chain.enabled && chain.running
				const funding = balances?.chains.find((row) => row.chainId === chain.meta.chainId)?.funding
				// The shortfall warning applies only before delegation: a delegated chain fills
				// regardless, and the paymaster draws the fee token down as it fills, so its
				// balance sitting under the recommended amount is expected, not actionable.
				const needsFunding = funding !== undefined && !funding.delegated && !funding.funded
				const checked = live ? funding?.delegated === true : chain.enabled
				const disabled = live
					? checked || !funding?.funded || enabling !== undefined
					: !chain.enabled && !chain.bundlerUrl
				return (
					<Collapsible.Root
						className="card chain-configuration"
						data-enabled={chain.enabled}
						key={chain.meta.chainId}
						open={panels.isOpen(chain.meta.chainId)}
						onOpenChange={(open) => panels.setOpen(chain.meta.chainId, open)}
					>
						<div
							className="chain-configuration-header"
							onClick={(e) => {
								if (isHeaderControl(e)) return
								panels.setOpen(chain.meta.chainId, !panels.isOpen(chain.meta.chainId))
							}}
						>
							<div className="chain-identity">
								<ChainLogo label={chain.meta.label} />
								<div>
									<h2>{chain.meta.label}</h2>
									{chain.enabled && !chain.running ? (
										<span className="chain-source">Applies after restart</span>
									) : null}
									{live ? (
										<span className="chain-source" data-funded={needsFunding ? false : undefined}>
											{enabling === chain.meta.chainId
												? "Enabling fills…"
												: fillsStatus(funding, chain.meta.chainId)}
											{needsFunding ? (
												<button
													type="button"
													className="chain-check-again"
													onClick={() => void checkAgain()}
												>
													Check again
												</button>
											) : null}
										</span>
									) : null}
									{fillsError?.chainId === chain.meta.chainId ? (
										<span className="chain-source chain-source-error">{fillsError.message}</span>
									) : null}
									{chain.bundlerUrl ? null : <span className="chain-source">Watch only</span>}
								</div>
							</div>
							<div className="chain-header-controls">
								<label
									className="chain-enable-toggle"
									title={
										chain.bundlerUrl
											? undefined
											: "Hyperbridge runs no bundler for this chain, so it can only be watched."
									}
								>
									<input
										type="checkbox"
										checked={checked}
										disabled={disabled}
										onChange={(e) => {
											if (live) return void enableFills(chain.meta.chainId)
											toggleChain(chain, e.target.checked)
										}}
									/>
									<span className="chain-enable-switch" aria-hidden="true" />
									<span>Enable fills</span>
								</label>
								<ChainCollapseTrigger label={chain.meta.label} />
							</div>
						</div>
						<Collapsible.Content className="chain-collapsible-content">
							<div className="chain-configuration-fields">
								{chain.rpcUrls.map((url, index) => (
									<label className="field" key={index}>
										<span className="field-label">
											{index === 0 ? "RPC endpoint" : "Backup RPC endpoint"}
											{index === 0 ? <span className="field-required">Required</span> : null}
										</span>
										<small>
											{index === 0
												? "Used to read the chain and find orders."
												: "A second provider protects against unavailable or incorrect data."}
										</small>
										<div className="row">
											<input
												type="text"
												value={url}
												required={index === 0}
												onChange={(e) =>
													patch(chain.meta.chainId, {
														rpcUrls: chain.rpcUrls.map((item, itemIndex) =>
															itemIndex === index ? e.target.value : item,
														),
														verificationState: undefined,
														verificationMessage: undefined,
													})
												}
											/>
											{index > 0 ? (
												<button
													type="button"
													aria-label="Remove backup RPC"
													onClick={() =>
														patch(chain.meta.chainId, {
															rpcUrls: chain.rpcUrls.filter(
																(_, itemIndex) => itemIndex !== index,
															),
															verificationState: undefined,
															verificationMessage: undefined,
														})
													}
												>
													✕
												</button>
											) : null}
										</div>
									</label>
								))}
								<div className="chain-backup-rpc">
									<button
										className="chain-add-backup-button"
										type="button"
										onClick={() =>
											patch(chain.meta.chainId, {
												rpcUrls: [...chain.rpcUrls, ""],
												verificationState: undefined,
												verificationMessage: undefined,
											})
										}
									>
										<span aria-hidden="true">+</span> Add backup RPC
									</button>
									<p className="chain-info-text">
										<span className="chain-info-icon" aria-hidden="true">
											i
										</span>
										<span>
											A backup lets Simplex compare providers before it acts on chain data.
										</span>
									</p>
								</div>
								<div className="chain-configuration-actions">
									<div className="chain-verification-control">
										<button
											type="button"
											disabled={
												!chain.rpcUrls[0]?.trim() || chain.verificationState === "checking"
											}
											onClick={() => verifyChain(chain)}
										>
											{chain.verificationState === "checking" ? "Verifying…" : "Verify endpoints"}
										</button>
										<EndpointVerificationStatus
											state={chain.verificationState}
											message={chain.verificationMessage}
										/>
									</div>
								</div>
							</div>
						</Collapsible.Content>
					</Collapsible.Root>
				)
			})}

			<div className="operator-chain-save">
				<div className="row">
					<button type="button" className="primary" disabled={!loaded} onClick={save}>
						Save chain settings
					</button>
					{saved ? <span className="badge warn">Restart Simplex to apply</span> : null}
					{message ? <span className="badge ok">{message}</span> : null}
					{error ? <span className="badge err">{error}</span> : null}
				</div>
				<p className="hint">
					Endpoint changes and newly added chains apply after a restart. On a running chain, Enable fills
					applies at once, once the wallet holds at least {MIN_FEE_TOKEN_BALANCE} of the gateway fee token
					there. Those {MIN_FEE_TOKEN_BALANCE} are kept for gas; fills spend what is above them.
				</p>
			</div>
		</div>
	)
}

/** One line on whether a running chain fills, or what the wallet still needs there. */
function fillsStatus(funding: ChainFunding | undefined, chainId: number): string {
	if (!funding) return "Checking the balance…"
	if (funding.delegated) return "Fills enabled"
	const shortfall = fundingShortfall(funding, nativeTokenSymbol(chainId))
	return shortfall ? `Needs ${shortfall}` : "Ready to enable fills"
}
