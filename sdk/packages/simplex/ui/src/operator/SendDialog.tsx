import { useId, useMemo, useState } from "react"
import { toast } from "sonner"
import { chainByChainId } from "@/cli/init/chains"
import { formatChainKey, parseChainKey } from "@/config/interpolated-curve"
import { api } from "../api"
import { ChainLogo } from "../components/ChainLogo"
import {
	CheckIcon,
	ChevronDownIcon,
	CloseIcon,
	DashedCircleIcon,
	ExternalLinkIcon,
	PasteIcon,
} from "../components/InterfaceIcons"
import { SendConfirmDialog, type SendSummary } from "../components/SendConfirmDialog"
import { TokenIcon } from "../components/TokenIcon"
import { type PickerToken, TokenNetworkPicker } from "../components/TokenNetworkPicker"
import { WizardDialog } from "../components/WizardDialog"
import { useAction } from "../lib/hooks"
import type { BalanceSnapshot, SendTokenOption } from "../types"
import { sendTokenOptionsForChain } from "./sendModel"

/**
 * What `/api/send` answers. A 200 carries a confirmed tx hash. A 202 means the send was
 * submitted but its confirmation outlived the wait, so it is still pending: a sponsored
 * send has only a `userOpHash` then, a native one has its `txHash`.
 */
type SendResult =
	| { pending?: false; txHash: string; redeemed: boolean }
	| { pending: true; sponsored: boolean; txHash?: string; userOpHash?: string; message: string }

interface SendBalance {
	label: string
	status: "loading" | "available" | "unavailable"
	/** Underlying held directly by the wallet; null when unknown. */
	wallet: number | null
	/** Spendable wallet plus withdrawable vault assets; null when unknown. */
	available: number | null
	/** Largest single vault that could cover a shortfall — withdrawals draw on one vault, not several. */
	largestVault: number
}

/** One token on one network that the wallet can send. */
interface SendAsset {
	key: string
	chain: string
	chainId: number
	symbol: string
	address: string
	balance: SendBalance
}

const PERCENTAGES = [25, 50, 75, 100] as const
/** Fraction digits a percentage fill keeps. It rounds down, so it never asks for more than is held. */
const PERCENTAGE_FRACTION_DIGITS = 6
const ADDRESS = /^0x[0-9a-fA-F]{40}$/

function percentageAmount(available: number, percentage: number): string {
	const scale = 10 ** PERCENTAGE_FRACTION_DIGITS
	const amount = Math.floor(((available * percentage) / 100) * scale) / scale
	return amount.toLocaleString("en-US", { useGrouping: false, maximumFractionDigits: PERCENTAGE_FRACTION_DIGITS })
}

function SendHash({ hash, explorerUrl }: { hash?: string; explorerUrl?: string }) {
	if (!hash) return null
	if (explorerUrl) {
		return (
			<a
				className="operator-send-tx-link"
				href={`${explorerUrl}/tx/${hash}`}
				target="_blank"
				rel="noreferrer"
				title={hash}
				aria-label={`Open transaction ${hash} on the block explorer in a new tab`}
			>
				<span className="mono">{hash.slice(0, 18)}…</span>
				<ExternalLinkIcon aria-hidden="true" />
			</a>
		)
	}
	return (
		<span className="mono" title={hash}>
			{hash.slice(0, 18)}…
		</span>
	)
}

/**
 * Sends tokens out of the filler wallet: the asset on its network, the recipient, then the
 * amount. Nothing is submitted until the review that follows is confirmed.
 */
export function SendDialog(props: {
	open: boolean
	onClose: () => void
	chains: number[]
	chainLabel: (id: number | string) => string
	sendTokens?: Record<string, SendTokenOption[]>
	balances?: BalanceSnapshot
	onSent: () => Promise<void> | void
}) {
	const { chains, chainLabel, sendTokens, balances } = props
	const [selectedKey, setSelectedKey] = useState<string>()
	const [pickerOpen, setPickerOpen] = useState(false)
	const [custom, setCustom] = useState(false)
	const [customToken, setCustomToken] = useState("")
	const [amount, setAmount] = useState("")
	const [to, setTo] = useState("")
	const [result, setResult] = useState<SendResult & { explorerUrl?: string }>()
	const [sending, setSending] = useState(false)
	const [review, setReview] = useState<SendSummary>()
	const { run: act, error } = useAction()
	const addressId = useId()
	const amountId = useId()

	const assets = useMemo<SendAsset[]>(
		() =>
			chains.flatMap((chainId) => {
				const chain = formatChainKey(chainId)
				return sendTokenOptionsForChain(sendTokens?.[chain], chainId).map((option) => ({
					key: `${chain}:${option.address}`,
					chain,
					chainId,
					symbol: option.symbol,
					address: option.address,
					balance: selectedSendBalance(balances, chain, option.address),
				}))
			}),
		[chains, sendTokens, balances],
	)
	const asset = assets.find((entry) => entry.key === selectedKey) ?? assets[0]
	const chain = asset?.chain ?? formatChainKey(chains[0] ?? "")
	const network = chainLabel(asset?.chainId ?? chain)
	const tokenAddress = custom ? customToken.trim() : (asset?.address ?? "")
	const symbol = custom ? "tokens" : (asset?.symbol ?? "")
	const balance = custom ? selectedSendBalance(balances, chain, tokenAddress) : (asset?.balance ?? UNKNOWN)

	const recipient = to.trim()
	const addressValid = ADDRESS.test(recipient)
	const requested = Number.parseFloat(amount.trim())
	const amountValid = /^\d*\.?\d+$/.test(amount.trim()) && requested > 0
	const exceeds = amountValid && balance.available !== null && requested > balance.available
	const ready = amountValid && !exceeds && addressValid && tokenAddress !== ""
	const canPaste = typeof navigator !== "undefined" && typeof navigator.clipboard?.readText === "function"

	const pickerTokens: PickerToken[] = assets.map((entry) => ({
		key: entry.key,
		chainId: entry.chainId,
		symbol: entry.symbol,
		name: entry.symbol,
		balance: entry.balance.available,
	}))

	/** Opens the review; nothing is submitted until it is confirmed. */
	const openReview = () => {
		const wallet = balance.wallet
		const covered = wallet === null ? null : Math.min(requested, wallet)
		setReview({
			amount: amount.trim(),
			symbol,
			native: tokenAddress === "native",
			chainLabel: network,
			to: recipient,
			wallet,
			available: balance.available,
			fromWallet: covered,
			fromVault: covered === null ? null : Math.max(0, requested - covered),
			largestVault: balance.largestVault,
		})
	}

	const send = () => {
		const chainId = parseChainKey(chain)
		const explorerUrl = chainId === null ? undefined : chainByChainId(chainId)?.explorerUrl
		setReview(undefined)
		setResult(undefined)
		setSending(true)
		return act(async () => {
			try {
				const res = await api.post<SendResult>("/api/send", {
					chain,
					token: tokenAddress,
					amount: amount.trim(),
					to: recipient,
				})
				setResult({ ...res, explorerUrl })
				// Clear the amount whatever the outcome: a send that came back pending (202) must
				// not be re-submitted by the next click. `act` gets no "Sent" label here because a
				// pending response sets `result`, and the success line already renders from it.
				setAmount("")
				await props.onSent()
			} finally {
				setSending(false)
			}
		})
	}

	const paste = async () => {
		try {
			setTo((await navigator.clipboard.readText()).trim())
		} catch {
			toast.error("Could not read the clipboard")
		}
	}

	return (
		<WizardDialog
			className="send-dialog"
			open={props.open}
			onClose={props.onClose}
			title="Send from filler wallet"
			description="Check the address and the network. A transfer cannot be undone."
		>
			<div className="send-form">
				<div className="send-asset">
					<button
						type="button"
						className="token-picker-trigger"
						aria-label="Choose token to send"
						onClick={() => setPickerOpen(true)}
						disabled={assets.length === 0}
					>
						<span className="send-review-asset">
							{custom ? <DashedCircleIcon aria-hidden="true" /> : <TokenIcon symbol={symbol} size="lg" />}
							<span className="send-review-chain-badge">
								<ChainLogo label={network} />
							</span>
						</span>
						<span className="token-picker-token-name">
							<strong>{custom ? "Custom token" : symbol}</strong>
							<span>{network}</span>
						</span>
						<span className="send-asset-balance mono" data-status={balance.status}>
							{balance.available === null ? balance.label : formatSendBalance(balance.available)}
						</span>
						<ChevronDownIcon aria-hidden="true" />
					</button>
					{custom ? (
						<label className="field">
							<span>Token contract on {network}</span>
							<input
								type="text"
								placeholder="0x…"
								value={customToken}
								onChange={(e) => setCustomToken(e.target.value)}
							/>
						</label>
					) : null}
					<button type="button" className="send-custom-toggle" onClick={() => setCustom((value) => !value)}>
						{custom ? "Choose a listed token" : "Token not listed? Send it by contract address"}
					</button>
				</div>

				<div className="send-address">
					<label htmlFor={addressId}>To address</label>
					<div className="send-address-field">
						<span className="send-address-status" aria-hidden="true">
							{addressValid ? <ChainLogo label={network} /> : <DashedCircleIcon />}
						</span>
						<input
							id={addressId}
							type="text"
							value={to}
							placeholder="Recipient address"
							autoComplete="off"
							autoCapitalize="off"
							spellCheck={false}
							onChange={(e) => setTo(e.target.value)}
						/>
						{canPaste ? (
							<button
								type="button"
								className="send-paste"
								aria-label="Paste from clipboard"
								onClick={() => void paste()}
							>
								<PasteIcon aria-hidden="true" />
							</button>
						) : null}
					</div>
					{recipient ? (
						<p className="send-address-note" data-state={addressValid ? "ok" : "error"}>
							{addressValid ? <CheckIcon aria-hidden="true" /> : <CloseIcon aria-hidden="true" />}
							{addressValid ? "Valid address" : "Enter a valid address"}
						</p>
					) : null}
				</div>

				<div className="send-amount" data-invalid={exceeds || undefined}>
					<div className="send-amount-head">
						<label htmlFor={amountId}>Amount</label>
						<span className="send-amount-chip">
							{custom ? null : <TokenIcon symbol={symbol} />}
							{custom ? "Custom token" : symbol}
						</span>
					</div>
					<input
						id={amountId}
						type="text"
						inputMode="decimal"
						placeholder="0.00"
						value={amount}
						aria-invalid={exceeds || undefined}
						onChange={(e) => setAmount(e.target.value.replace(/[^0-9.]/g, ""))}
					/>
					{exceeds ? <p className="send-amount-error">More than the {balance.label} available</p> : null}
					<div className="send-amount-foot">
						<small className="operator-send-balance" data-status={balance.status} aria-live="polite">
							Available <strong>{balance.label}</strong>
						</small>
						<div className="send-percentages" role="group" aria-label="Share of the available balance">
							{PERCENTAGES.map((percentage) => {
								const fill = balance.available
									? percentageAmount(balance.available, percentage)
									: undefined
								return (
									<button
										key={percentage}
										type="button"
										disabled={!fill}
										aria-pressed={fill !== undefined && fill === amount.trim()}
										onClick={() => fill && setAmount(fill)}
									>
										{percentage === 100 ? "Max" : `${percentage}%`}
									</button>
								)
							})}
						</div>
					</div>
				</div>

				{tokenAddress === "native" ? (
					<p className="hint">Keep some {symbol} for gas on networks without a paymaster.</p>
				) : null}
				{result && !result.pending && (
					<p className="hint">
						✓ Sent{result.redeemed && " (topped up from the vault)"} — tx{" "}
						<SendHash hash={result.txHash} explorerUrl={result.explorerUrl} />
					</p>
				)}
				{result?.pending && (
					// A pending send is not a success, and its message is worded by the server for the
					// sponsored (no tx hash) or native case. The hash is shown only when there is one.
					<p className="warning">
						⚠ {result.message}
						{(result.txHash ?? result.userOpHash) && (
							<>
								{" — "}
								<SendHash
									hash={result.txHash ?? result.userOpHash}
									explorerUrl={result.txHash ? result.explorerUrl : undefined}
								/>
							</>
						)}
					</p>
				)}
				{error && <p className="error">{error}</p>}

				<div className="send-actions">
					<button type="button" className="secondary" onClick={props.onClose} disabled={sending}>
						Cancel
					</button>
					<button type="button" className="primary" disabled={!ready || sending} onClick={openReview}>
						{sending ? "Sending…" : "Send"}
					</button>
				</div>
			</div>

			<TokenNetworkPicker
				open={pickerOpen}
				onClose={() => setPickerOpen(false)}
				title="Choose token to send"
				networks={chains.map((chainId) => ({ chainId, label: chainLabel(chainId) }))}
				tokens={pickerTokens}
				selectedKey={custom ? undefined : asset?.key}
				loading={!balances || balances.status === "loading"}
				onSelect={(token) => {
					setSelectedKey(token.key)
					setCustom(false)
					setPickerOpen(false)
				}}
			/>
			{review && (
				<SendConfirmDialog
					open
					summary={review}
					sending={sending}
					onConfirm={() => void send()}
					onCancel={() => setReview(undefined)}
				/>
			)}
		</WizardDialog>
	)
}

const UNKNOWN_BALANCE = { wallet: null, available: null, largestVault: 0 } as const
const UNKNOWN: SendBalance = { label: "—", status: "loading", ...UNKNOWN_BALANCE }

function selectedSendBalance(balances: BalanceSnapshot | undefined, chain: string, token: string): SendBalance {
	if (!balances || balances.status === "loading") return { label: "Loading…", status: "loading", ...UNKNOWN_BALANCE }

	const chainId = parseChainKey(chain)
	const chainBalance = balances.chains.find((row) => row.chainId === chainId)
	if (!chainBalance) return { label: "Unavailable", status: "unavailable", ...UNKNOWN_BALANCE }

	if (token === "native") {
		// Native has no vault behind it: the wallet is the whole balance.
		return chainBalance.native
			? {
					label: `${formatSendBalance(chainBalance.native.amount)} ${chainBalance.native.symbol}`,
					status: "available",
					wallet: chainBalance.native.amount,
					available: chainBalance.native.amount,
					largestVault: 0,
				}
			: { label: "Unavailable", status: "unavailable", ...UNKNOWN_BALANCE }
	}

	if (!token) return UNKNOWN
	const asset = chainBalance.assets.find((row) => row.address.toLowerCase() === token.toLowerCase())
	if (asset?.available === null || asset?.available === undefined) {
		return { label: "Unavailable", status: "unavailable", ...UNKNOWN_BALANCE }
	}
	return {
		label: `${formatSendBalance(asset.available)} ${asset.symbol}`,
		status: "available",
		wallet: asset.wallet,
		available: asset.available,
		largestVault: asset.vaults.reduce((most, vault) => Math.max(most, vault.available), 0),
	}
}

function formatSendBalance(value: number): string {
	return value.toLocaleString(undefined, { maximumFractionDigits: 4 })
}
