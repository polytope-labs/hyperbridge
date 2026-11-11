import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { formatAmount } from "../lib/format"
import type { BalanceSnapshot, StatusOperator } from "../types"
import { ReturnMetric } from "./analytics/ReturnMetric"
import { LimitOrders } from "./LimitOrders"
import { availableStablecoinLiquidity, OperatorBalances } from "./OperatorBalances"

export function OperatorOverview(props: {
	status: StatusOperator
	balances: BalanceSnapshot | undefined
	/** Pause/resume and stop, rendered inline so the page that shows health also controls it. */
	runtime: { pending: boolean; onTogglePause: () => void; onStop: () => void }
	onOpenAnalytics: () => void
}) {
	const { status, balances, runtime, onOpenAnalytics } = props
	const stablecoinLiquidity = availableStablecoinLiquidity(balances)
	const nativeSetups = (balances?.chains ?? []).flatMap((row) =>
		row.nativeSetup
			? [{ ...row.nativeSetup, chainId: row.chainId, symbol: row.native?.symbol ?? "native gas" }]
			: [],
	)

	return (
		<div className="operator-overview">
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

		</div>
	)
}

function Metric(props: { label: string; value: string }) {
	return (
		<div>
			<span>{props.label}</span>
			<strong>{props.value}</strong>
		</div>
	)
}
