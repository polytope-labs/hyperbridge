import { useState } from "react"
import {
	availableStablecoinLiquidity as aggregateStablecoinLiquidity,
	sumAvailableStablecoins,
} from "@/services/stablecoin-liquidity"
import { AppSelect, type AppSelectOption } from "../components/AppSelect"
import { ChainLogo } from "../components/ChainLogo"
import { TokenIcon } from "../components/TokenIcon"
import { formatAmount } from "../lib/format"
import type { BalanceSnapshot, StatusOperator } from "../types"

type ChainBalances = BalanceSnapshot["chains"][number]
type AssetBalance = ChainBalances["assets"][number]
type SnapshotStatus = BalanceSnapshot["status"]

/** The network the balances were last narrowed to, kept across pages and reloads. */
const NETWORK_KEY = "simplex.balances.network"

function storedNetwork(): number | null {
	try {
		const chainId = Number(localStorage.getItem(NETWORK_KEY))
		return Number.isInteger(chainId) && chainId > 0 ? chainId : null
	} catch {
		// Storage can be unavailable (private windows, locked-down webviews).
		return null
	}
}

function storeNetwork(chainId: number): void {
	try {
		localStorage.setItem(NETWORK_KEY, String(chainId))
	} catch {
		// Remembering is a convenience; the switcher works without it.
	}
}

/**
 * Liquidity, one network at a time. A nine-chain config stacked every network's token grid
 * down one page, so the switcher narrows the cards to a single network. A token the network
 * holds none of gets no card; one whose balance could not be read still does.
 */
export function OperatorBalances(props: { status: StatusOperator; balances: BalanceSnapshot | undefined }) {
	const { status, balances } = props
	const chains = balances?.chains ?? []
	const snapshot: SnapshotStatus = balances?.status ?? "loading"

	// A remembered network that is no longer enabled falls back to the first, as before.
	const [picked, setPicked] = useState<number | null>(storedNetwork)
	const selected = chains.find((row) => row.chainId === picked) ?? chains[0]
	const pick = (chainId: number) => {
		setPicked(chainId)
		storeNetwork(chainId)
	}

	const label = selected ? chainLabel(status, selected.chainId) : ""
	// Known to be zero, not merely unread: a failed read is null and keeps its card, so a
	// problem never reads as an empty wallet.
	const held = selected?.assets.filter((asset) => asset.total !== 0) ?? []

	return (
		<section className="operator-section">
			<div className="operator-section-heading">
				<div>
					<span className="eyebrow">Liquidity</span>
					<h2>Balances</h2>
				</div>
				<div className="operator-balance-heading-aside">
					{selected ? (
						<div className="operator-network-switcher">
							<AppSelect
								value={String(selected.chainId)}
								onValueChange={(value) => pick(Number(value))}
								ariaLabel="Network to show balances for"
								caption={`${chains.length} ${chains.length === 1 ? "network" : "networks"}`}
								contentClassName="operator-network-menu"
								header={
									<>
										<span>Network</span>
										<span>Stables available</span>
									</>
								}
								options={chains.map((row) => networkOption(row, status, snapshot))}
							/>
						</div>
					) : null}
					<small>
						{balances?.updatedAt
							? `Updated ${new Date(balances.updatedAt).toLocaleTimeString()}`
							: "Awaiting first refresh"}
					</small>
				</div>
			</div>

			{selected ? (
				<>
					<div className="operator-balance-focus">
						<span className="operator-balance-focus-network">
							Showing <strong>{label}</strong> only ·{" "}
							{status.watchOnly[selected.chainId] ? "Observe only" : "Filling enabled"}
						</span>
						<BalanceValue
							label="Network gas"
							value={
								selected.native
									? `${selected.native.amount.toFixed(4)} ${selected.native.symbol}`
									: "Unavailable"
							}
						/>
					</div>

					{balances && balances.status !== "fresh" && balances.issues.length > 0 ? (
						<div className="operator-balance-notice" role="status">
							<strong>Some balances are unavailable</strong>
							<span>
								Simplex did not estimate missing values. The next refresh will retry{" "}
								{balances.issues.length} failed {balances.issues.length === 1 ? "read" : "reads"}.
							</span>
						</div>
					) : null}

					<div className="operator-asset-balances" aria-label={`${label} token balances`}>
						{held.map((asset) => (
							<AssetBalanceCard asset={asset} key={asset.address} />
						))}
						{selected.assets.length === 0 ? (
							<span className="operator-balance-missing">No tracked tokens</span>
						) : held.length === 0 ? (
							<span className="operator-balance-missing">{`Nothing held on ${label}`}</span>
						) : null}
					</div>
				</>
			) : (
				<p className="operator-empty">Balances will appear after the first refresh.</p>
			)}
		</section>
	)
}

function networkOption(row: ChainBalances, status: StatusOperator, snapshot: SnapshotStatus): AppSelectOption {
	const label = chainLabel(status, row.chainId)
	const stables = availableStablecoins(row.assets, snapshot)
	return {
		value: String(row.chainId),
		label,
		leading: <ChainLogo label={label} />,
		description: status.watchOnly[row.chainId] ? "Observe only" : "Filling enabled",
		trailing: stables === null ? "Unavailable" : formatStables(stables),
	}
}

/** Whole dollars once the figure is large enough for cents to be noise in a narrow column. */
function formatStables(value: number): string {
	return `$${value.toLocaleString(undefined, { maximumFractionDigits: value >= 1000 ? 0 : 2 })}`
}

function BalanceValue(props: { label: string; value: string }) {
	return (
		<div className="operator-balance-value">
			<span>{props.label}</span>
			<strong>{props.value}</strong>
		</div>
	)
}

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
				<BalancePart
					label="In vault"
					value={asset.vaultPosition}
					note={asset.vaults.some((vault) => !vault.acceptsDeposits) ? "Deposits closed" : undefined}
				/>
			</dl>

			<div className="operator-asset-available">
				<span>
					<i aria-hidden="true" />
					Available to fill
				</span>
				<strong>
					{asset.available === null ? "Unavailable" : `${formatAmount(asset.available)} ${asset.symbol}`}
				</strong>
			</div>
		</article>
	)
}

function BalancePart(props: { label: string; value: number | null; note?: string }) {
	return (
		<div>
			<dt>{props.label}</dt>
			<dd>
				{props.value === null ? "Unavailable" : formatAmount(props.value)}
				{props.note && <small className="operator-balance-note">{props.note}</small>}
			</dd>
		</div>
	)
}

function chainLabel(status: StatusOperator, chainId: number): string {
	return status.chainLabels?.[String(chainId)] ?? `Chain ${chainId}`
}

/**
 * Available USD stablecoins across `assets`, or null when any contributing read failed. Refusing
 * to estimate is the point: a sum quietly missing a leg understates liquidity without saying so.
 */
export function availableStablecoins(assets: AssetBalance[], snapshot: SnapshotStatus): number | null {
	if (snapshot === "loading") return null
	return sumAvailableStablecoins(assets)
}

export function availableStablecoinLiquidity(balances: BalanceSnapshot | undefined): number | null {
	return aggregateStablecoinLiquidity(balances)
}
