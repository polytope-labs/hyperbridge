import { formatAmount } from "../lib/format"
import type { AdminStrategyDto, BalanceSnapshot, ConfigDto, StatusOperator } from "../types"
import { availableStablecoinLiquidity, OperatorBalances } from "./OperatorBalances"
import { OperatorMarkets } from "./OperatorMarkets"

export function OperatorOverview(props: {
	status: StatusOperator
	balances: BalanceSnapshot | undefined
	strategies: AdminStrategyDto[]
	config: ConfigDto | undefined
	onResetHalt: () => void
	onMarketsChanged: () => Promise<void>
	/** Pause/resume and stop, rendered inline so the page that shows health also controls it. */
	runtime: { pending: boolean; onTogglePause: () => void; onStop: () => void }
}) {
	const { status, balances, strategies, config, onResetHalt, onMarketsChanged, runtime } = props
	const stablecoinLiquidity = availableStablecoinLiquidity(balances)

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

			<section className="operator-metrics" aria-label="Runtime summary">
				<Metric label="Enabled networks" value={String(status.chains.length)} />
				<Metric
					label="Active markets"
					value={String(strategies.filter((strategy) => !strategy.referenceOnly).length)}
				/>
				<Metric
					label="Available liquidity"
					value={stablecoinLiquidity === null ? "—" : `$${formatAmount(stablecoinLiquidity)}`}
				/>
				<Metric label="BRIDGE available" value={balances?.hyperbridge?.free.toLocaleString() ?? "—"} />
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

			<OperatorMarkets
				strategies={strategies}
				config={config}
				chains={status.chains}
				chainLabels={status.chainLabels}
				solverAddress={status.addresses?.evm}
				onChanged={onMarketsChanged}
			/>
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
