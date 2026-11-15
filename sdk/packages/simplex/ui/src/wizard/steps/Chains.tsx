import * as Collapsible from "@radix-ui/react-collapsible"
import { useCallback, useRef } from "react"
import { nativeTokenSymbol } from "@/cli/init/chains"
import { fundingShortfall, MIN_FEE_TOKEN_BALANCE } from "@/services/chain-funding"
import { api } from "../../api"
import { ChainCollapseTrigger, useChainPanels } from "../../components/ChainPanel"
import { ChainLogo } from "../../components/ChainLogo"
import { CopyHash } from "../../components/CopyHash"
import { DownloadIcon, LockIcon, ToggleIcon } from "../../components/InterfaceIcons"
import { formatAmount } from "../../lib/format"
import { usePolling } from "../../lib/hooks"
import type { ChainFunding } from "../../types"
import { loadOrderbook } from "../orderbook"
import { patchChain, walletAddress, type ChainDraft, type WizardState } from "../state"
import { StepPoints, type StepPoint } from "../StepPoints"
import type { StepProps } from "../Wizard"

/** How often balances are re-read, so a deposit unlocks its chain without a reload. */
const FUNDING_POLL_MS = 15_000

/** What funding a chain takes. A chain that also needs native for a one-time setup says so on its own row. */
const FUNDING_POINTS: ReadonlyArray<StepPoint> = [
	{
		icon: DownloadIcon,
		title: "Send USDC to your wallet",
		detail: `At least ${MIN_FEE_TOKEN_BALANCE} on each chain you want to fill on. BNB Chain takes USDT.`,
	},
	{
		icon: LockIcon,
		title: `${MIN_FEE_TOKEN_BALANCE} is kept for gas`,
		detail: "Simplex fills with what you hold above it.",
	},
	{
		icon: ToggleIcon,
		title: "Then turn fills on",
		detail: "A chain's switch unlocks once its funds arrive.",
	},
]

export function StepChains({ state, setState }: StepProps) {
	const panels = useChainPanels()
	const wallet = walletAddress(state)
	useChainFunding(state, setState)

	const patch = (chainId: number, changes: Partial<ChainDraft>) => setState((s) => patchChain(s, chainId, changes))

	return (
		<div className="wizard-sections chains-step">
			{state.orderbookError ? (
				<div className="card">
					<p className="error">{state.orderbookError}</p>
					<button type="button" onClick={() => void loadOrderbook(setState)}>
						Retry
					</button>
				</div>
			) : null}

			<section className="card chain-funding-intro" aria-label="Fund the filler wallet">
				<StepPoints label="How funding a chain works" points={FUNDING_POINTS} />
				{wallet ? (
					<>
						<h2>Your filler wallet</h2>
						<div className="review-account-value">
							<CopyHash value={wallet} chars={42} copyLabel="Copy filler wallet address" />
						</div>
					</>
				) : null}
			</section>

			{state.chains.map((chain) => {
				// Turning fills off is always allowed; turning them on waits for the funds.
				const canToggle = chain.enabled || chain.funding?.funded === true
				// A chain that fills and has run short is a problem. One that was never funded is just not in use.
				const runShort = chain.enabled && chain.funding?.funded === false
				return (
					<Collapsible.Root
						className="card chain-configuration"
						data-enabled={chain.enabled}
						data-locked={canToggle ? undefined : true}
						key={chain.meta.chainId}
						open={panels.isOpen(chain.meta.chainId)}
						onOpenChange={(open) => panels.setOpen(chain.meta.chainId, open)}
					>
						<div className="chain-configuration-header">
							<div className="chain-identity">
								<ChainLogo label={chain.meta.label} />
								<div>
									<h2>{chain.meta.label}</h2>
									<span className="chain-source" data-funded={runShort ? false : undefined}>
										{fundingStatus(chain)}
									</span>
								</div>
							</div>
							<div className="chain-header-controls">
								<label
									className="chain-enable-toggle"
									title={canToggle ? undefined : "Fund the wallet on this chain first"}
								>
									<input
										type="checkbox"
										checked={chain.enabled}
										disabled={!canToggle}
										onChange={(e) => patch(chain.meta.chainId, { enabled: e.target.checked })}
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
											{index === 0 ? "RPC endpoint" : `RPC endpoint ${index + 1}`}
											{index === 0 ? <span className="field-required">Required</span> : null}
										</span>
										{index === 0 && <small>Used to read the chain and find orders.</small>}
										{index === 1 && (
											<small>
												Reads are agreed across every endpoint listed, so a wrong or unavailable
												answer from any one of them cannot mislead the filler.
											</small>
										)}
										<div className="row">
											<input
												type="text"
												style={{ flex: 1 }}
												value={url}
												required={index === 0}
												onChange={(e) =>
													patch(chain.meta.chainId, {
														rpcUrls: chain.rpcUrls.map((u, i) =>
															i === index ? e.target.value : u,
														),
													})
												}
											/>
											{index > 0 && (
												<button
													type="button"
													onClick={() =>
														patch(chain.meta.chainId, {
															rpcUrls: chain.rpcUrls.filter((_, i) => i !== index),
														})
													}
												>
													✕
												</button>
											)}
										</div>
									</label>
								))}
								<div className="chain-backup-rpc">
									<button
										className="chain-add-backup-button"
										type="button"
										title="A backup RPC lets Simplex compare independent providers before it acts on chain data."
										onClick={() =>
											patch(chain.meta.chainId, {
												rpcUrls: [...chain.rpcUrls, ""],
											})
										}
									>
										<span aria-hidden="true">+</span>
										Add backup RPC
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
							</div>
						</Collapsible.Content>
					</Collapsible.Root>
				)
			})}
		</div>
	)
}

/** One line on what the wallet holds toward filling on `chain`. */
function fundingStatus(chain: ChainDraft): string {
	if (!chain.funding) return chain.fundingError ? "Could not read the balance" : "Checking the balance…"
	const shortfall = fundingShortfall(chain.funding, nativeTokenSymbol(chain.meta.chainId))
	if (shortfall) return chain.enabled ? `Needs ${shortfall}` : `Send ${shortfall}`
	const held = `${formatAmount(chain.funding.feeToken.balance)} ${chain.funding.feeToken.symbol}`
	return chain.enabled ? held : `${held}, ready`
}

/**
 * Re-reads every chain's funding while the step is open. Each read goes through the
 * chain's current endpoints, so an edited RPC is picked up on the next pass.
 */
function useChainFunding(state: WizardState, setState: StepProps["setState"]) {
	const wallet = walletAddress(state)
	const chains = useRef(state.chains)
	chains.current = state.chains
	const reading = useRef(false)

	const read = useCallback(async () => {
		if (!wallet || reading.current) return
		reading.current = true
		try {
			await Promise.all(
				chains.current.map(async ({ meta, rpcUrls }) => {
					const urls = rpcUrls.map((url) => url.trim()).filter(Boolean)
					if (urls.length === 0) return
					try {
						const funding = await api.post<ChainFunding>("/api/setup/chain-funding", {
							chainId: meta.chainId,
							rpcUrls: urls,
							address: wallet,
						})
						setState((s) => patchChain(s, meta.chainId, { funding, fundingError: undefined }))
					} catch (err) {
						const fundingError = err instanceof Error ? err.message : String(err)
						setState((s) => patchChain(s, meta.chainId, { fundingError }))
					}
				}),
			)
		} finally {
			reading.current = false
		}
	}, [wallet, setState])
	usePolling(read, FUNDING_POLL_MS)
}
