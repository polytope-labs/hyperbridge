import { useCallback, useState } from "react"
import { chainByChainId } from "@/cli/init/chains"
import { api } from "../api"
import { ChainLogo } from "../components/ChainLogo"
import { CopyHash } from "../components/CopyHash"
import { ExternalLinkIcon } from "../components/InterfaceIcons"
import { TokenIcon } from "../components/TokenIcon"
import { formatClockTime, formatDate, formatDecimalAmount, formatTokenAmount, shortAddress } from "../lib/format"
import { usePolling } from "../lib/hooks"
import type { BalanceSnapshot, LedgerLeg, WalletTxDto } from "../types"
import { WalletTools } from "./WalletTools"

const KIND_LABEL: Record<WalletTxDto["kind"], string> = {
	send: "Send",
	sweep: "Vault sweep",
	redeem: "Vault redeem",
	fill: "Order fill",
}

/** One stroke icon per action; the colour comes from the cell's data-kind. */
function KindIcon({ kind }: { kind: WalletTxDto["kind"] }) {
	const paths: Record<WalletTxDto["kind"], string> = {
		// a receipt: the order, settled
		fill: "M4 2.5h8v11l-2-1.3-2 1.3-2-1.3-2 1.3v-11ZM6 6h4M6 8.5h4",
		// sent up into the vault
		sweep: "M8 13.5v-8m0 0 3 3m-3-3-3 3M3 2.5h10",
		// comes back down out of the vault
		redeem: "M8 2.5v8m0 0 3-3m-3 3-3-3M3 13.5h10",
		// away from the wallet
		send: "M3.5 12.5 12.5 3.5m0 0H6m6.5 0V10",
	}
	return (
		<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
			<path d={paths[kind]} />
		</svg>
	)
}

/** A signed amount with its token logo; vault shares wear a vault badge over the underlying's logo. */
function Leg(props: { leg: LedgerLeg; sign: "in" | "out" }) {
	const { leg, sign } = props
	const text = leg.decimals === null ? formatDecimalAmount(leg.amount) : formatTokenAmount(leg.amount, leg.decimals)
	const full = leg.decimals === null ? leg.amount : formatTokenAmount(leg.amount, leg.decimals, leg.decimals)
	return (
		<span className="ledger-delta" data-sign={sign} title={`${full} ${leg.symbol}`}>
			<span className="ledger-token" data-vault={leg.vault || undefined} aria-hidden="true">
				<TokenIcon symbol={leg.icon} size="sm" />
				{/* A bank: the vault the shares represent. */}
				{leg.vault && (
					<svg className="ledger-vault-badge" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
						<path d="M2.5 6.5 8 3l5.5 3.5H2.5ZM4 6.5v5M8 6.5v5M12 6.5v5M2.5 13.5h11" />
					</svg>
				)}
			</span>
			<span>
				<span className="ledger-delta-sign">{sign === "in" ? "+" : "−"}</span>
				{text} {leg.symbol}
			</span>
		</span>
	)
}

function AmountCell(props: { leg: LedgerLeg | null; sign: "in" | "out"; note?: string | null }) {
	const { leg, sign, note } = props
	if (!leg) return <span className="ledger-quiet">{note ?? "—"}</span>
	return (
		<span className="ledger-move">
			<Leg leg={leg} sign={sign} />
			{note && <span className="ledger-counterparty">{note}</span>}
		</span>
	)
}

/** Where a send went. Vault rows say it with the share token on the other side, so they get nothing. */
function counterpartyOf(tx: WalletTxDto): string | null {
	if (tx.kind !== "send" || !tx.to) return null
	return `to ${shortAddress(tx.to)}`
}

function TxLink(props: { tx: WalletTxDto }) {
	const { tx } = props
	const explorer = tx.chainId !== null ? chainByChainId(tx.chainId)?.explorerUrl : undefined
	if (!explorer) return <CopyHash value={tx.txHash} chars={14} />
	return (
		<span className="history-links ledger-links">
			<a
				href={`${explorer}/tx/${tx.txHash}`}
				target="_blank"
				rel="noreferrer"
				title={`View ${shortAddress(tx.txHash, 8, 4)} on the block explorer`}
				aria-label="View transaction on the block explorer"
			>
				<ExternalLinkIcon aria-hidden="true" />
			</a>
		</span>
	)
}

export function Wallet(props: {
	chains: number[]
	chainLabels?: Record<string, string>
	balances?: BalanceSnapshot
	onBalancesChanged: () => Promise<void> | void
	onOpenChains: () => void
}) {
	const [txs, setTxs] = useState<WalletTxDto[]>()
	const [error, setError] = useState<string>()

	const load = useCallback(async () => {
		try {
			const history = await api.get<{ txs: WalletTxDto[] }>("/api/wallet/history?limit=200")
			setTxs(history.txs)
			setError(undefined)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		}
	}, [])
	usePolling(load, 30_000)

	const chainLabel = (id: number | null) => (id === null ? "—" : (props.chainLabels?.[String(id)] ?? `chain ${id}`))

	return (
		<div className="operator-page-content">
			<WalletTools
				chains={props.chains}
				chainLabels={props.chainLabels}
				balances={props.balances}
				onBalancesChanged={props.onBalancesChanged}
				onOpenChains={props.onOpenChains}
			/>
			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Ledger</span>
						<h2>Transaction history</h2>
					</div>
					<small>{txs ? `${txs.length} recorded` : "Loading"}</small>
				</div>
				{txs?.length === 0 && <p className="operator-empty">No transactions recorded yet.</p>}
				{txs && txs.length > 0 && (
					<div style={{ overflowX: "auto" }}>
						<table className="history-table ledger-table">
							<thead>
								<tr>
									<th>Action</th>
									<th>Amount in</th>
									<th>Amount out</th>
									<th>Chain</th>
									<th>Tx</th>
									<th>Time</th>
								</tr>
							</thead>
							<tbody>
								{txs.map((tx) => (
									<tr key={tx.id}>
										<td>
											<span className="ledger-action" data-kind={tx.kind}>
												<span className="ledger-icon" aria-hidden="true">
													<KindIcon kind={tx.kind} />
												</span>
												<span className="ledger-action-copy">
													<strong>{KIND_LABEL[tx.kind]}</strong>
												</span>
											</span>
										</td>
										<td>
											<AmountCell leg={tx.in} sign="in" note={tx.kind === "redeem" ? counterpartyOf(tx) : null} />
										</td>
										<td>
											<AmountCell leg={tx.out} sign="out" note={tx.kind === "redeem" ? null : counterpartyOf(tx)} />
										</td>
										<td>
											{tx.chainId !== null ? (
												<span className="ledger-chain">
													<ChainLogo label={chainLabel(tx.chainId)} />
													<span>{chainLabel(tx.chainId)}</span>
												</span>
											) : (
												<span className="ledger-quiet">—</span>
											)}
										</td>
										<td>
											<TxLink tx={tx} />
										</td>
										<td>
											<span className="history-time">
												<strong>{formatClockTime(tx.ts)}</strong>
												<small>{formatDate(tx.ts)}</small>
											</span>
										</td>
									</tr>
								))}
							</tbody>
						</table>
					</div>
				)}
			</section>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
