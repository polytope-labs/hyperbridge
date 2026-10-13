import { Tooltip, TooltipContent, TooltipTrigger } from "@hyperbridge/ui"
import { CircleInfo } from "@hyperbridge/ui/icons"
import { useCallback, useEffect, useMemo, useState } from "react"
import { chainByChainId } from "@/cli/init/chains"
import { api } from "../api"
import { ChainLogo } from "../components/ChainLogo"
import { ChevronDownIcon, RefreshIcon } from "../components/InterfaceIcons"
import { TokenIcon } from "../components/TokenIcon"
import { type PickerToken, TokenNetworkPicker } from "../components/TokenNetworkPicker"
import type { TimelineTopUp } from "./topUpTimelineModel"
import { WizardDialog } from "../components/WizardDialog"
import { formatAmount } from "../lib/format"

/** Preset spends, in dollars of the chosen stablecoin. */
const PRESETS = ["5", "10"] as const
const QUOTE_DEBOUNCE_MS = 400

type PayToken = "USDC" | "USDT"

/** A stablecoin the filler wallet can pay with, on any supported network. */
interface PaySource {
	chain: string
	chainId: number
	token: PayToken
	address: string
	/** Human-readable balance, or null when the network could not be read. */
	balance: string | null
	/** The balance less the paymaster's gas reserve; what a top-up can spend. */
	spendable: string | null
	reserve: string
}

const TOKEN_NAMES: Record<PayToken, string> = { USDC: "USD Coin", USDT: "Tether USD" }

interface TopUpQuote {
	amountIn: string
	bridgeOut: string
	rate: string
	fillable: boolean
	maxFillableIn: string
}

export interface BridgeTopUpState extends TimelineTopUp {
	id: string
	token: PayToken
	amountIn: string
	bridgeOut: string
	/** True while the top-up blocks a new one: its order is open, unfilled or being reclaimed. */
	pending: boolean
}

function sourceKey(source: { chainId: number; token: string }): string {
	return `${source.chainId}:${source.token}`
}

function networkLabel(chainId: number): string {
	return chainByChainId(chainId)?.label ?? `Chain ${chainId}`
}


/**
 * Buys BRIDGE for the filler's Hyperbridge account. The filler pays USDC or USDT from any
 * running network; the order fills on BSC, where its output call sends the BRIDGE on to
 * Hyperbridge through the HyperFungibleToken. Once the order is placed, its progress moves
 * to {@link BridgeTopUpProgressDialog}.
 */
export function BridgeTopUpDialog(props: {
	open: boolean
	onClose: () => void
	/** Called with the top-up once its order is on its way. */
	onStarted: (topUp: BridgeTopUpState) => void
}) {
	const { open, onClose, onStarted } = props
	const [sources, setSources] = useState<PaySource[]>()
	const [sourcesError, setSourcesError] = useState<string>()
	const [selectedKey, setSelectedKey] = useState<string>()
	const [pickerOpen, setPickerOpen] = useState(false)
	const [amount, setAmount] = useState<string>(PRESETS[0])
	const [quote, setQuote] = useState<TopUpQuote>()
	const [quoteError, setQuoteError] = useState<string>()
	const [quoting, setQuoting] = useState(false)
	const [submitting, setSubmitting] = useState(false)
	const [submitError, setSubmitError] = useState<string>()
	// Balances on every network the top-up can pay from, read fresh each time the dialog opens
	// and on demand. A refresh keeps showing the previous balances until the new ones land.
	const [refreshing, setRefreshing] = useState(false)
	const loadSources = useCallback(async () => {
		setRefreshing(true)
		setSourcesError(undefined)
		try {
			const { sources: next } = await api.get<{ sources: PaySource[] }>("/api/bridge-top-up/sources")
			setSources(next)
		} catch (err) {
			setSourcesError(err instanceof Error ? err.message : String(err))
		} finally {
			setRefreshing(false)
		}
	}, [])
	useEffect(() => {
		if (open) void loadSources()
	}, [open, loadSources])

	const pickerTokens = useMemo<PickerToken[]>(
		() =>
			(sources ?? []).map((s) => {
				const balance = s.balance === null ? null : Number(s.balance)
				return {
					key: sourceKey(s),
					chainId: s.chainId,
					symbol: s.token,
					name: TOKEN_NAMES[s.token],
					balance,
					// Both are dollar stablecoins; their balance is their dollar value.
					usd: balance,
				}
			}),
		[sources],
	)
	const pickerNetworks = useMemo(
		() =>
			[...new Set((sources ?? []).map((s) => s.chainId))].map((chainId) => ({
				chainId,
				label: networkLabel(chainId),
			})),
		[sources],
	)
	// Defaults to the largest balance anywhere, so an operator with funds on one network lands on it.
	const richest = useMemo(
		() => [...(sources ?? [])].sort((a, b) => Number(b.balance ?? -1) - Number(a.balance ?? -1))[0],
		[sources],
	)
	const source = sources?.find((s) => sourceKey(s) === selectedKey) ?? richest
	const sourceBalance = source?.spendable === null || source === undefined ? null : Number(source.spendable)
	const sourceReserve = source ? Number(source.reserve) : 0
	const amountValue = Number(amount)
	const validAmount = amount.trim() !== "" && Number.isFinite(amountValue) && amountValue > 0
	const insufficient = sourceBalance !== null && validAmount && amountValue > sourceBalance

	useEffect(() => {
		if (!open || !source || !validAmount) {
			setQuote(undefined)
			setQuoteError(undefined)
			return
		}
		let cancelled = false
		setQuoting(true)
		const timer = setTimeout(async () => {
			try {
				const query = new URLSearchParams({ chain: source.chain, token: source.token, amount: amount.trim() })
				const next = await api.get<TopUpQuote>(`/api/bridge-top-up/quote?${query}`)
				if (cancelled) return
				setQuote(next)
				setQuoteError(undefined)
			} catch (err) {
				if (cancelled) return
				setQuote(undefined)
				setQuoteError(err instanceof Error ? err.message : String(err))
			} finally {
				if (!cancelled) setQuoting(false)
			}
		}, QUOTE_DEBOUNCE_MS)
		return () => {
			cancelled = true
			clearTimeout(timer)
		}
	}, [open, source?.chain, source?.token, amount, validAmount])

	const close = () => {
		setSubmitError(undefined)
		onClose()
	}

	const submit = async () => {
		if (!source) return
		setSubmitting(true)
		setSubmitError(undefined)
		try {
			const state = await api.post<BridgeTopUpState>("/api/bridge-top-up", {
				chain: source.chain,
				token: source.token,
				amount: amount.trim(),
			})
			onStarted(state)
		} catch (err) {
			setSubmitError(err instanceof Error ? err.message : String(err))
		} finally {
			setSubmitting(false)
		}
	}

	const canBuy = !!source && validAmount && !insufficient && !!quote?.fillable && !quoting && !submitting

	return (
		<WizardDialog
			className="bridge-top-up-dialog"
			open={open}
			onClose={close}
			title="Top up BRIDGE"
			description="Buy BRIDGE with USDC or USDT from the filler wallet. It is delivered to your Hyperbridge account for bid fees."
		>
			<div className="send-review bridge-top-up">
				<>
						<div className="field">
							<span>Pay with</span>
							<button
								type="button"
								className="token-picker-trigger"
								onClick={() => setPickerOpen(true)}
								disabled={!sources?.length}
							>
								{!sources ? (
									<span className="token-picker-loading">
										<span className="inline-spinner" aria-hidden="true" />
										Reading balances…
									</span>
								) : source ? (
									<>
										<span className="send-review-asset">
											<TokenIcon symbol={source.token} size="lg" />
											<span className="send-review-chain-badge">
												<ChainLogo label={networkLabel(source.chainId)} />
											</span>
										</span>
										<span className="token-picker-token-name">
											<strong>
												{TOKEN_NAMES[source.token]} <small>{source.token}</small>
											</strong>
											<span>{networkLabel(source.chainId)}</span>
										</span>
									</>
								) : (
									<span className="token-picker-loading">No network to pay from</span>
								)}
								<ChevronDownIcon aria-hidden="true" />
							</button>
						</div>
						<div className="field">
							<span className="field-label">
								{/* A wrapping <label> would hand its hover and clicks to the info button, the first labelable element inside. */}
								<label htmlFor="bridge-top-up-amount">Amount</label>
								{!sources ? (
									<small className="operator-send-balance">
										<span className="inline-spinner" role="status" aria-label="Reading balance" />
									</small>
								) : (
									<small className="operator-send-balance bridge-top-up-balances">
										{sourceReserve > 0 && source && (
											<Tooltip>
												<TooltipTrigger asChild>
													<button type="button" className="vault-info-trigger" aria-label="Why spendable is lower">
														<CircleInfo aria-hidden="true" />
													</button>
												</TooltipTrigger>
												<TooltipContent className="simplex-tooltip-content" side="top" align="start" sideOffset={7}>
													<p>
														{formatAmount(sourceReserve)} {source.token} stays in the wallet to pay gas, which the
														paymaster charges in {source.token}.
													</p>
												</TooltipContent>
											</Tooltip>
										)}
										<span>
											Balance <strong>{source?.balance == null ? "—" : formatAmount(Number(source.balance))}</strong>
										</span>
										<span className="bridge-top-up-balances-divider" aria-hidden="true" />
										<span>
											Spendable{" "}
											<strong>
												{source && sourceBalance !== null ? `${formatAmount(sourceBalance)} ${source.token}` : "—"}
											</strong>
										</span>
										<button
											type="button"
											className="vault-info-trigger bridge-top-up-refresh"
											aria-label="Refresh balances"
											title="Refresh balances"
											data-refreshing={refreshing || undefined}
											disabled={refreshing}
											onClick={() => void loadSources()}
										>
											<RefreshIcon aria-hidden="true" />
										</button>
									</small>
								)}
							</span>
							<input
								id="bridge-top-up-amount"
								type="text"
								inputMode="decimal"
								placeholder="0.00"
								value={amount}
								onChange={(e) => setAmount(e.target.value)}
								aria-invalid={insufficient || undefined}
								aria-describedby={insufficient ? "bridge-top-up-amount-error" : undefined}
							/>
							{insufficient && source && (
								<small id="bridge-top-up-amount-error" className="field-error">
									Not enough {source.token} on {networkLabel(source.chainId)}
								</small>
							)}
						</div>
						<div className="bridge-top-up-presets" role="group" aria-label="Preset amounts">
							{PRESETS.map((preset) => (
								<button
									key={preset}
									type="button"
									className={`secondary${amount.trim() === preset ? " active" : ""}`}
									aria-pressed={amount.trim() === preset}
									onClick={() => setAmount(preset)}
								>
									${preset}
								</button>
							))}
						</div>

						<dl className="send-review-facts">
							<div>
								<dt>You receive</dt>
								<dd>
									{quoting ? "Quoting…" : quote?.fillable ? `≈${formatAmount(Number(quote.bridgeOut))} BRIDGE` : "—"}
								</dd>
							</div>
							<div>
								<dt>Rate</dt>
								<dd>
									{quote?.fillable && source
										? `1 ${source.token} ≈ ${formatAmount(Number(quote.rate))} BRIDGE`
										: "—"}
								</dd>
							</div>
						</dl>

						{sourcesError && <p className="error">{sourcesError}</p>}
						{sources?.length === 0 && (
							<p className="hint">
								Top-ups pay from the networks the filler fills on. Enable filling on a network to pay from it.
							</p>
						)}
						{quote && !quote.fillable && (
							<p className="error">
								The orderbook can fill up to {formatAmount(Number(quote.maxFillableIn))} {source?.token} right now.
							</p>
						)}
						{quoteError && <p className="error">{quoteError}</p>}
						{submitError && <p className="error">{submitError}</p>}

						<div className="send-review-actions">
							<button type="button" className="secondary" onClick={close} disabled={submitting}>
								Cancel
							</button>
							<button type="button" className="primary" onClick={() => void submit()} disabled={!canBuy}>
								{submitting ? "Placing order…" : "Buy BRIDGE"}
							</button>
						</div>
				</>
			</div>
			<TokenNetworkPicker
				open={pickerOpen}
				onClose={() => setPickerOpen(false)}
				title="Pay with"
				networks={pickerNetworks}
				tokens={pickerTokens}
				selectedKey={source ? sourceKey(source) : undefined}
				loading={!sources}
				onSelect={(token) => {
					setSelectedKey(token.key)
					setPickerOpen(false)
				}}
			/>
		</WizardDialog>
	)
}
