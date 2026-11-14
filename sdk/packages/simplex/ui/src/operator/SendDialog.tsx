import { useId, useMemo, useState } from "react"
import { toast } from "sonner"
import { chainByChainId } from "@/cli/init/chains"
import { formatChainKey, parseChainKey } from "@/config/interpolated-curve"
import { api } from "../api"
import { AmountInput, groupThousands } from "../components/AmountInput"
import { ChainLogo } from "../components/ChainLogo"
import { CheckIcon, CloseIcon, DashedCircleIcon, ExternalLinkIcon, PasteIcon } from "../components/InterfaceIcons"
import { SendReview, type SendSummary } from "../components/SendReview"
import { TokenIcon } from "../components/TokenIcon"
import { WizardDialog } from "../components/WizardDialog"
import { useAction } from "../lib/hooks"
import type { BalanceSnapshot, SendTokenOption } from "../types"
import { type SendAssetOption, SendAssetSelect } from "./SendAssetSelect"
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

/** What a finished send shows: the amount as it was typed, its token and its network. */
interface SentSummary {
	amount: string
	symbol: string
	network: string
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
 * amount. The review replaces the form in the same dialog, and nothing is submitted until it
 * is confirmed.
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
	const [custom, setCustom] = useState(false)
	const [customToken, setCustomToken] = useState("")
	const [amount, setAmount] = useState("")
	const [to, setTo] = useState("")
	const [result, setResult] = useState<SendResult & { explorerUrl?: string; sent: SentSummary }>()
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

	const balanceText = balance.available === null ? balance.label : formatSendBalance(balance.available)
	const currentAsset: SendAssetOption = {
		key: custom ? "" : (asset?.key ?? ""),
		symbol: custom || !asset ? null : asset.symbol,
		network,
		balance: balanceText,
		funded: (balance.available ?? 0) > 0,
	}
	/** Every sendable token, largest balance first. A balance that is not known yet sorts last. */
	const assetOptions = useMemo<SendAssetOption[]>(
		() =>
			[...assets]
				.sort((a, b) => (b.balance.available ?? -1) - (a.balance.available ?? -1))
				.map((entry) => ({
					key: entry.key,
					symbol: entry.symbol,
					network: chainLabel(entry.chainId),
					balance:
						entry.balance.available === null
							? entry.balance.label
							: formatSendBalance(entry.balance.available),
					funded: (entry.balance.available ?? 0) > 0,
				})),
		[assets, chainLabel],
	)

	/** Shows the review in place of the form; nothing is submitted until it is confirmed. */
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
				setResult({ ...res, explorerUrl, sent: { amount: amount.trim(), symbol, network } })
				// Clear the amount whatever the outcome: a send that came back pending (202) must
				// not be re-submitted by the next click. `act` gets no "Sent" label here because a
				// pending response sets `result`, and the success screen already renders from it.
				setAmount("")
				await props.onSent()
			} finally {
				// The review stays up while the send is in flight, then gives way to the outcome.
				setReview(undefined)
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

	const success = result && !result.pending ? result : undefined
	/** A finished send and an open review are cleared on the way out, so the dialog opens on the form again. */
	const close = () => {
		if (success) setResult(undefined)
		setReview(undefined)
		props.onClose()
	}

	return (
		<WizardDialog
			className="send-dialog"
			open={props.open}
			onClose={close}
			title={review ? "Review transfer" : "Send from filler wallet"}
			description={
				review
					? "Check the destination and the amount. Transfers cannot be undone."
					: "Check the address and the network. A transfer cannot be undone."
			}
		>
			{review ? (
				<SendReview
					summary={review}
					sending={sending}
					onConfirm={() => void send()}
					onBack={() => setReview(undefined)}
				/>
			) : null}
			{success ? (
				<div className="send-success" role="status">
					<span className="send-success-mark" aria-hidden="true">
						<CheckIcon />
					</span>
					<div>
						<h3>Funds sent</h3>
						<p>
							{groupThousands(success.sent.amount)} {success.sent.symbol} sent on {success.sent.network}
							{success.redeemed ? ", topped up from the vault" : ""}.
						</p>
					</div>
					{success.explorerUrl ? (
						<a
							className="send-success-link"
							href={`${success.explorerUrl}/tx/${success.txHash}`}
							target="_blank"
							rel="noreferrer"
							title={success.txHash}
						>
							View transaction
						</a>
					) : (
						<SendHash hash={success.txHash} />
					)}
					<div className="send-actions" data-single>
						<button type="button" className="primary" onClick={close}>
							Done
						</button>
					</div>
				</div>
			) : null}
			<div className="send-form" hidden={Boolean(success) || Boolean(review)}>
				<SendAssetSelect
					current={currentAsset}
					options={assetOptions}
					onSelect={(key) => {
						setSelectedKey(key)
						setCustom(false)
					}}
				/>
				{/* Everything the open asset list covers. */}
				<div className="send-fields">
					<div className="send-custom">
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
						<button
							type="button"
							className="send-custom-toggle"
							onClick={() => setCustom((value) => !value)}
						>
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
						<AmountInput id={amountId} value={amount} invalid={exceeds} onValueChange={setAmount} />
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
				</div>

				<div className="send-actions">
					<button type="button" className="secondary" onClick={close} disabled={sending}>
						Cancel
					</button>
					<button type="button" className="primary" disabled={!ready || sending} onClick={openReview}>
						Send
					</button>
				</div>
			</div>
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
