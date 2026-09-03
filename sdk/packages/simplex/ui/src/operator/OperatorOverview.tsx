import { ChainLogo } from "../components/ChainLogo"
import { TokenIcon } from "../components/TokenIcon"
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
				{balances && balances.status !== "fresh" && balances.issues.length > 0 ? (
					<div className="operator-balance-notice" role="status">
						<strong>Some balances are unavailable</strong>
						<span>
							Simplex did not estimate missing values. The next refresh will retry {balances.issues.length}{" "}
							failed {balances.issues.length === 1 ? "read" : "reads"}.
						</span>
					</div>
				) : null}
				<div className="operator-balance-list">
					{balances?.chains.map((row) => {
						const label = status.chainLabels?.[String(row.chainId)] ?? `Chain ${row.chainId}`
						return (
							<div className="operator-balance-row" key={row.chainId}>
								<div className="operator-balance-network">
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
										label="Network gas"
										value={row.native ? `${row.native.amount.toFixed(4)} ${row.native.symbol}` : "Unavailable"}
									/>
								</div>
								<div className="operator-asset-balances" aria-label={`${label} token balances`}>
									{row.assets.map((asset) => (
										<AssetBalanceCard asset={asset} key={asset.address} />
									))}
									{row.assets.length === 0 ? <span className="operator-balance-missing">No tracked tokens</span> : null}
								</div>
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

type AssetBalance = BalanceSnapshot["chains"][number]["assets"][number]

function AssetBalanceCard({ asset }: { asset: AssetBalance }) {
	const token = asset.symbol.trim().toLowerCase()
	const statusLabel =
		asset.status === "fresh" ? "Live balance" : asset.status === "partial" ? "Partial data" : "Unavailable"

	return (
		<article
			className="operator-asset-balance"
			data-token={token}
			data-status={asset.status}
			aria-label={`${asset.symbol} balance`}
		>
			<header className="operator-asset-header">
				<div className="operator-token-identity">
					<span className="operator-token-icon">
						<TokenIcon symbol={asset.symbol} size="lg" />
					</span>
					<span>
						<strong>{asset.symbol}</strong>
						<small>{statusLabel}</small>
					</span>
				</div>
				<div className="operator-asset-total">
					<span>Total balance</span>
					<strong>{asset.total === null ? "—" : formatAmount(asset.total)}</strong>
				</div>
			</header>

			<dl className="operator-asset-breakdown">
				<BalancePart label="In wallet" value={asset.wallet} />
				<BalancePart label="In vault" value={asset.vaultPosition} />
			</dl>

			<div className="operator-asset-available">
				<span>
					<i aria-hidden="true" />
					Available to fill
				</span>
				<strong>{asset.available === null ? "Unavailable" : `${formatAmount(asset.available)} ${asset.symbol}`}</strong>
			</div>
		</article>
	)
}

function BalancePart(props: { label: string; value: number | null }) {
	return (
		<div>
			<dt>{props.label}</dt>
			<dd>{props.value === null ? "Unavailable" : formatAmount(props.value)}</dd>
		</div>
	)
}

function availableStablecoinLiquidity(balances: BalanceSnapshot | undefined): number | null {
	if (!balances || balances.status === "loading") return null
	const stables = balances.chains.flatMap((chain) =>
		chain.assets.filter((asset) => asset.symbol.toUpperCase() === "USDC" || asset.symbol.toUpperCase() === "USDT"),
	)
	if (stables.length === 0) return balances.status === "fresh" ? 0 : null
	if (stables.some((asset) => asset.available === null)) return null
	return stables.reduce((total, asset) => total + (asset.available ?? 0), 0)
}

function formatAmount(value: number): string {
	return value.toLocaleString(undefined, { maximumFractionDigits: 4 })
}
