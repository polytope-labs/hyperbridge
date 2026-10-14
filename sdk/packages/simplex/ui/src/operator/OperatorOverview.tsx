import { type ReactNode, useCallback, useRef, useState } from "react"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { usePolling } from "../lib/hooks"
import { formatAmount, formatCompactAmount } from "../lib/format"
import type { BalanceSnapshot, StatusOperator } from "../types"
import { ReturnMetric } from "./analytics/ReturnMetric"
import { BridgeTopUpDialog, type BridgeTopUpState } from "./BridgeTopUpDialog"
import { BridgeTopUpProgressDialog } from "./BridgeTopUpProgressDialog"
import { LimitOrders } from "./LimitOrders"
import { availableStablecoinLiquidity, OperatorBalances } from "./OperatorBalances"

export function OperatorOverview(props: {
	status: StatusOperator
	balances: BalanceSnapshot | undefined
	onResetHalt: () => void
	/** Pause/resume and stop, rendered inline so the page that shows health also controls it. */
	runtime: { pending: boolean; onTogglePause: () => void; onStop: () => void }
	onOpenAnalytics: () => void
	/** Reloads the balance snapshot after a BRIDGE top-up fills. */
	onBalancesChanged: () => void
}) {
	const { status, balances, onResetHalt, runtime, onOpenAnalytics, onBalancesChanged } = props
	const [topUpOpen, setTopUpOpen] = useState(false)
	const [progressOpen, setProgressOpen] = useState(false)
	const [topUp, setTopUp] = useState<BridgeTopUpState>()
	const lastTopUpStage = useRef<string>(undefined)
	// A request in flight is not overlapped: a sync reads chains and the indexer and can take seconds.
	const topUpLoading = useRef(false)
	const syncTopUp = useRef(false)
	// A delivered top-up may still be waiting for the indexer's last statuses; the solver decides.
	syncTopUp.current = progressOpen
	const loadTopUp = useCallback(async () => {
		if (topUpLoading.current) return
		topUpLoading.current = true
		try {
			// While the progress dialog shows a pending top-up, the solver first re-reads it from the
			// chains and the indexer: another party may have filled or delivered it.
			const path = syncTopUp.current ? "/api/bridge-top-up?sync=1" : "/api/bridge-top-up"
			const { current } = await api.get<{ current: BridgeTopUpState | null }>(path)
			// The balance only moves once the BRIDGE reaches Hyperbridge; reload it then rather than on the next poll.
			const previous = lastTopUpStage.current
			lastTopUpStage.current = current?.stage
			if (current?.stage === "delivered" && previous !== undefined && previous !== "delivered") onBalancesChanged()
			setTopUp(current ?? undefined)
		} catch {
			// Older solvers and fillers without a Hyperbridge key have no top-up; the button stays plain.
		} finally {
			topUpLoading.current = false
		}
	}, [onBalancesChanged])
	// Tracked closely while the dialog shows it, otherwise only to spot one left pending.
	usePolling(loadTopUp, progressOpen ? 5_000 : topUp?.pending ? 15_000 : 30_000)
	const stablecoinLiquidity = availableStablecoinLiquidity(balances)
	const nativeSetups = (balances?.chains ?? []).flatMap((row) =>
		row.nativeSetup
			? [{ ...row.nativeSetup, chainId: row.chainId, symbol: row.native?.symbol ?? "native gas" }]
			: [],
	)

	return (
		<div className="operator-overview">
			{status.halted.length > 0 ? (
				<div className="operator-alert" data-tone="error">
					<div>
						<strong>Overfill protection needs attention</strong>
						<p>
							Strategy {status.halted.map((index) => `#${index}`).join(", ")} is halted. Inspect its venue
							before resuming.
						</p>
					</div>
					<button type="button" onClick={onResetHalt}>
						Reset halt
					</button>
				</div>
			) : null}

			{nativeSetups.map((setup) => {
				const label = status.chainLabels?.[String(setup.chainId)] ?? `Chain ${setup.chainId}`
				const amount = setup.required.toLocaleString(undefined, { maximumSignificantDigits: 2 })
				return (
					<div className="operator-alert" data-tone="warning" key={setup.chainId}>
						<div>
							<strong>
								{label} needs about {amount} {setup.symbol} to finish setup
							</strong>
							<p>
								Its {setup.feeToken} can't pay for its own first approval, so that one transaction is
								paid in {setup.symbol}. The paymaster covers gas after that.
							</p>
						</div>
						{status.addresses ? (
							<CopyHash value={status.addresses.evm} copyLabel="Copy filler wallet address" />
						) : null}
					</div>
				)
			})}

			<section className="operator-metrics" aria-label="Runtime summary">
				<Metric label="Enabled networks" value={String(status.chains.length)} />
				<Metric
					label="Available liquidity"
					value={stablecoinLiquidity === null ? "—" : `$${formatAmount(stablecoinLiquidity)}`}
				/>
				<Metric
					label="BRIDGE available"
					value={balances?.hyperbridge ? formatCompactAmount(balances.hyperbridge.free) : "—"}
					title={balances?.hyperbridge ? `${balances.hyperbridge.free.toLocaleString()} BRIDGE` : undefined}
					action={
						balances?.hyperbridge ? (
							<button
								type="button"
								className="secondary operator-metric-action"
								data-pending={topUp?.pending ? "" : undefined}
								// The tile is narrow; the pending pill says the short form and names the long one.
								aria-label={topUp?.pending ? "Top up pending" : undefined}
								title={topUp?.pending ? "Top up pending" : undefined}
								onClick={() => (topUp?.pending ? setProgressOpen(true) : setTopUpOpen(true))}
							>
								{topUp?.pending ? "Pending" : "Top up"}
							</button>
						) : undefined
					}
				/>
				<ReturnMetric onOpen={onOpenAnalytics} />
			</section>

			<section className="operator-section operator-runtime" aria-label="Runtime controls">
				<div className="operator-runtime-state">
					<span className={`operator-status-dot ${status.paused ? "warn" : ""}`} />
					<div>
						<strong>{status.paused ? "New fills are paused" : "Filling is active"}</strong>
						<p>
							{status.paused
								? "Order monitoring continues, but new fills are not analysed. The pause persists across restarts."
								: "Simplex is monitoring and filling eligible orders. Pausing lets in-flight fills complete."}
						</p>
					</div>
				</div>
				<div className="operator-runtime-actions">
					<button type="button" className="primary" onClick={runtime.onTogglePause} disabled={runtime.pending}>
						{status.paused ? "Resume filling" : "Pause new fills"}
					</button>
					<button
						type="button"
						className="operator-runtime-stop"
						onClick={runtime.onStop}
						disabled={runtime.pending}
						title="Drain in-flight fills, unwind eligible vault positions, and exit the process"
					>
						Stop filler
					</button>
				</div>
			</section>

			<OperatorBalances status={status} balances={balances} />

			<LimitOrders chains={status.chains} chainLabels={status.chainLabels} balances={balances} />

			<BridgeTopUpDialog
				open={topUpOpen}
				onClose={() => setTopUpOpen(false)}
				onStarted={(started) => {
					setTopUp(started)
					setTopUpOpen(false)
					setProgressOpen(true)
					void loadTopUp()
				}}
			/>

			<BridgeTopUpProgressDialog
				open={progressOpen}
				onOpenChange={setProgressOpen}
				topUp={topUp}
				onChanged={() => void loadTopUp()}
			/>
		</div>
	)
}

function Metric(props: { label: string; value: string; title?: string; action?: ReactNode }) {
	return (
		<div>
			{/* An action sits beside the label, so the figure keeps the tile's full width as it grows. */}
			{props.action ? (
				<div className="operator-metric-label">
					<span>{props.label}</span>
					{props.action}
				</div>
			) : (
				<span>{props.label}</span>
			)}
			<strong title={props.title}>{props.value}</strong>
		</div>
	)
}
