import { useCallback, useState } from "react"
import { chainByChainId } from "@/cli/init/chains"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { usePolling } from "../lib/hooks"
import type { BalanceSnapshot, WalletTxDto } from "../types"
import { WalletTools } from "./WalletTools"

const KIND_LABEL: Record<WalletTxDto["kind"], string> = {
	send: "send",
	sweep: "vault sweep",
	redeem: "vault redeem",
	fill: "order fill",
}

function describe(tx: WalletTxDto): string {
	if (tx.kind !== "send") return KIND_LABEL[tx.kind]
	const what = [tx.amount, tx.token].filter(Boolean).join(" ")
	return tx.to ? `send ${what} to ${tx.to.slice(0, 10)}…` : `send ${what}`
}

function TxLink(props: { tx: WalletTxDto }) {
	const { tx } = props
	const explorer = tx.chainId !== null ? chainByChainId(tx.chainId)?.explorerUrl : undefined
	if (!explorer) return <CopyHash value={tx.txHash} chars={14} />
	return (
		<span className="row" style={{ gap: "0.3rem", display: "inline-flex" }}>
			<a className="mono" href={`${explorer}/tx/${tx.txHash}`} target="_blank" rel="noreferrer" title={tx.txHash}>
				{tx.txHash.slice(0, 14)}… ↗
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
					<table>
						<thead>
							<tr>
								<th>Time</th>
								<th>Chain</th>
								<th>Action</th>
								<th>Tx</th>
							</tr>
						</thead>
						<tbody>
							{txs.map((tx) => (
								<tr key={tx.id}>
									<td>{new Date(tx.ts).toLocaleString()}</td>
									<td>{chainLabel(tx.chainId)}</td>
									<td>
										{describe(tx)}
										{tx.sponsored && <span className="badge"> sponsored</span>}
									</td>
									<td>
										<TxLink tx={tx} />
									</td>
								</tr>
							))}
						</tbody>
					</table>
				)}
			</section>
			{error && <p className="error">{error}</p>}
		</div>
	)
}
