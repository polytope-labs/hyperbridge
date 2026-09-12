import { ChainLogo } from "./ChainLogo"
import { TokenIcon } from "./TokenIcon"
import { WizardDialog } from "./WizardDialog"

/** What the operator is about to send, and where the funds come from. */
export interface SendSummary {
	/** As typed, so the review shows exactly what will be submitted. */
	amount: string
	symbol: string
	native: boolean
	chainLabel: string
	to: string
	/** Underlying held directly by the wallet; null when the balance could not be read. */
	wallet: number | null
	/** Spendable wallet plus withdrawable vault assets; null when unknown. */
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
	return value === null ? "—" : value.toLocaleString(undefined, { maximumFractionDigits: 4 })
}

/**
 * Review step for an outbound transfer. Everything here is read from the
 * balance snapshot the dashboard already polls, so it costs no extra call; the
 * transfer itself is only submitted on confirm.
 */
export function SendConfirmDialog(props: {
	open: boolean
	summary: SendSummary
	sending: boolean
	onConfirm: () => void
	onCancel: () => void
}) {
	const { summary, sending, onConfirm, onCancel } = props
	const fromVault = summary.fromVault ?? 0
	const shortOfVault = fromVault > 0 && fromVault > summary.largestVault

	return (
		<WizardDialog
			open={props.open}
			onClose={onCancel}
			title="Review transfer"
			description="Check the destination and the amount. Transfers cannot be undone."
		>
			<div className="send-review">
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
							{summary.amount} {summary.symbol}
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
								The transfer needs {amountText(fromVault)} {summary.symbol} from a vault, and the
								largest one can release {amountText(summary.largestVault)}. Withdrawals come from one
								vault, so this will fail. Send less, or redeem from the vaults first.
							</p>
						</div>
					</div>
				)}

				<div className="send-review-actions">
					<button type="button" className="secondary" onClick={onCancel} disabled={sending}>
						Cancel
					</button>
					<button type="button" className="primary" onClick={onConfirm} disabled={sending}>
						{sending ? "Sending…" : `Send ${summary.amount} ${summary.symbol}`}
					</button>
				</div>
			</div>
		</WizardDialog>
	)
}
