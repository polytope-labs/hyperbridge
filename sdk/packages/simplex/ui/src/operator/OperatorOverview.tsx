import { ChainLogo } from "../components/ChainLogo"
import type { AdminStrategyDto, BalanceSnapshot, ConfigDto, StatusOperator } from "../types"
import { OperatorMarkets } from "./OperatorMarkets"

export function OperatorOverview(props: {
	status: StatusOperator
	balances: BalanceSnapshot | undefined
	strategies: AdminStrategyDto[]
	config: ConfigDto | undefined
	onResetHalt: () => void
	onMarketsChanged: () => Promise<void>
}) {
	const { status, balances, strategies, config, onResetHalt, onMarketsChanged } = props
	const stablecoinLiquidity =
		balances?.chains.reduce((total, chain) => total + (chain.usdc ?? 0) + (chain.usdt ?? 0), 0) ?? 0

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
				<Metric label="Stablecoin liquidity" value={`$${stablecoinLiquidity.toLocaleString()}`} />
				<Metric label="BRIDGE available" value={balances?.hyperbridge?.free.toLocaleString() ?? "—"} />
			</section>

			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Liquidity</span>
						<h2>Balances by network</h2>
					</div>
					<small>
						{balances?.updatedAt
							? `Updated ${new Date(balances.updatedAt).toLocaleTimeString()}`
							: "Awaiting first refresh"}
					</small>
				</div>
				<div className="operator-balance-list">
					{balances?.chains.map((row) => {
						const label = status.chainLabels?.[String(row.chainId)] ?? `Chain ${row.chainId}`
						return (
							<div className="operator-balance-row" key={row.chainId}>
								<div className="operator-chain-name">
									<ChainLogo label={label} />
									<span>
										<strong>{label}</strong>
										<small>
											{status.watchOnly[row.chainId] ? "Observe only" : "Filling enabled"}
										</small>
									</span>
								</div>
								<BalanceValue
									label="Gas"
									value={row.native ? `${row.native.amount.toFixed(4)} ${row.native.symbol}` : "—"}
								/>
								<BalanceValue label="USDC" value={row.usdc?.toLocaleString() ?? "—"} />
								<BalanceValue label="USDT" value={row.usdt?.toLocaleString() ?? "—"} />
								<BalanceValue
									label="Other"
									value={
										row.exotics
											?.map((asset) => `${asset.amount.toLocaleString()} ${asset.symbol}`)
											.join(", ") || "—"
									}
								/>
							</div>
						)
					})}
					{!balances?.chains.length ? (
						<p className="operator-empty">Balances will appear after the first refresh.</p>
					) : null}
				</div>
			</section>

			<OperatorMarkets
				strategies={strategies}
				config={config}
				chains={status.chains}
				chainLabels={status.chainLabels}
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

function BalanceValue(props: { label: string; value: string }) {
	return (
		<div className="operator-balance-value">
			<span>{props.label}</span>
			<strong>{props.value}</strong>
		</div>
	)
}
