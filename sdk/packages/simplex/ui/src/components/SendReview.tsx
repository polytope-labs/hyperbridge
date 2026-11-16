import { useEffect, useRef, useState } from "react"
import { ChainLogo } from "./ChainLogo"
import { TokenIcon } from "./TokenIcon"
import { groupThousands } from "./AmountInput"

/** How long the confirm button stays off after the review appears. */
const CONFIRM_DELAY_MS = 500

/** What the operator is about to send, and where the funds come from. */
export interface SendSummary {
	/** The chain key and token that are submitted: a contract address, or "native". */
	chain: string
	token: string
	/** Set for a token sent by contract address, which has no ticker to recognise it by. */
	contract?: string
	/** As typed, so the review shows exactly what will be submitted. */
	amount: string
	symbol: string
	native: boolean
	chainLabel: string
	to: string
	/** Underlying held directly by the wallet; null when the balance could not be read. */
	wallet: number | null
	/** The most one send can move: the wallet plus what the largest vault can return. Null when unknown. */
	available: number | null
	/** The split the sender will make: the wallet covers what it can, the vault the rest. */
	fromWallet: number | null
	fromVault: number | null
	/**
	 * Largest single vault that could cover the shortfall. The sender withdraws
	 * from one vault, not several, so a shortfall larger than this fails even
	 * when the vaults add up to it.
	 */
	largestVault: number
}

function amountText(value: number | null): string {
	return value === null ? "—" : value.toLocaleString("en-US", { maximumFractionDigits: 4 })
}

/**
 * Review step of the send dialog. Everything here is read from the balance
 * snapshot the dashboard already polls, so it costs no extra call; the transfer
 * itself is only submitted on confirm.
 */
export function SendReview(props: {
	summary: SendSummary
	sending: boolean
	onConfirm: () => void
	onBack: () => void
}) {
	const { summary, sending, onConfirm, onBack } = props
	const fromVault = summary.fromVault ?? 0
	const shortOfVault = fromVault > 0 && fromVault > summary.largestVault
	const container = useRef<HTMLDivElement>(null)
	// The review takes the form's place, and its confirm button lands where Send was. A second tap
	// meant for Send must not confirm a transfer nobody has read.
	const [confirmable, setConfirmable] = useState(false)
	useEffect(() => {
		container.current?.focus()
		const timer = setTimeout(() => setConfirmable(true), CONFIRM_DELAY_MS)
		return () => clearTimeout(timer)
	}, [])

	return (
		<div ref={container} className="send-review" tabIndex={-1}>
			<div className="send-review-headline">
				{/* The network rides on the token as a badge, the way wallets show it. */}
				<span className="send-review-asset">
					<TokenIcon symbol={summary.symbol} size="lg" />
					<span className="send-review-chain-badge">
						<ChainLogo label={summary.chainLabel} />
					</span>
				</span>
				<div>
					<strong>
						{groupThousands(summary.amount)} {summary.symbol}
						{summary.native && " (native)"}
					</strong>
					<span className="send-review-network">on {summary.chainLabel}</span>
				</div>
			</div>

			<dl className="send-review-facts">
				<div className="send-review-wide">
					<dt>Recipient</dt>
					<dd className="mono send-review-address">{summary.to}</dd>
				</div>
				{summary.contract ? (
					<div className="send-review-wide">
						<dt>Token contract</dt>
						<dd className="mono send-review-address">{summary.contract}</dd>
					</div>
				) : null}
				<div>
					<dt>From wallet</dt>
					<dd>
						{amountText(summary.fromWallet)} {summary.symbol}
					</dd>
				</div>
				<div>
					<dt>From vault</dt>
					<dd>
						{fromVault > 0 ? (
							<>
								{amountText(fromVault)} {summary.symbol}
							</>
						) : (
							<span className="ledger-quiet">Not needed</span>
						)}
					</dd>
				</div>
				<div>
					<dt>Wallet balance</dt>
					<dd>
						{amountText(summary.wallet)} {summary.symbol}
					</dd>
				</div>
				<div>
					<dt>Available to send</dt>
					<dd>
						{amountText(summary.available)} {summary.symbol}
					</dd>
				</div>
			</dl>

			{fromVault > 0 && !shortOfVault && (
				<p className="hint">
					The wallet is short, so the difference is withdrawn from the vault in the same transaction.
				</p>
			)}
			{shortOfVault && (
				<div className="operator-alert send-review-alert">
					<div>
						<strong>No single vault covers the shortfall.</strong>
						<p>
							The transfer needs {amountText(fromVault)} {summary.symbol} from a vault, and the largest
							one can release {amountText(summary.largestVault)}. Withdrawals come from one vault, so this
							will fail. Send less, or redeem from the vaults first.
						</p>
					</div>
				</div>
			)}

			<div className="send-actions">
				<button type="button" className="secondary" onClick={onBack} disabled={sending}>
					Back
				</button>
				<button type="button" className="primary" onClick={onConfirm} disabled={sending || !confirmable}>
					{sending ? "Sending…" : `Send ${groupThousands(summary.amount)} ${summary.symbol}`}
				</button>
			</div>
		</div>
	)
}
