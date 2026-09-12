import { type CSSProperties, useState } from "react"
import { AppSelect, type AppSelectOption } from "../components/AppSelect"
import { ChainLogo } from "../components/ChainLogo"
import { TokenIcon } from "../components/TokenIcon"
import { formatAmount } from "../lib/format"
import type { BalanceSnapshot, StatusOperator } from "../types"

type ChainBalances = BalanceSnapshot["chains"][number]
type AssetBalance = ChainBalances["assets"][number]
type SnapshotStatus = BalanceSnapshot["status"]

/** Tokens past this are named in the truncation note rather than dropped in silence. */
const MAX_TOTAL_CELLS = 4

const STABLES = new Set(["USDC", "USDT"])

/**
 * Liquidity, one network at a time. A nine-chain config stacked every network's token grid
 * down one page, so the switcher narrows the cards to a single network — and the totals strip
 * above them keeps the cross-network figures the stacked view gave away for free.
 */
export function OperatorBalances(props: { status: StatusOperator; balances: BalanceSnapshot | undefined }) {
	const { status, balances } = props
	const chains = balances?.chains ?? []
	const snapshot: SnapshotStatus = balances?.status ?? "loading"

	const [picked, setPicked] = useState<number | null>(null)
	const selected = chains.find((row) => row.chainId === picked) ?? chains[0]

	const label = selected ? chainLabel(status, selected.chainId) : ""
	const totals = selected ? tokenTotals(chains, selected.chainId) : []
	const cells = totals.slice(0, MAX_TOTAL_CELLS)
	const dropped = totals.slice(MAX_TOTAL_CELLS)

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
								onValueChange={(value) => setPicked(Number(value))}
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
					<div className="operator-balance-totals-heading">
						<span>Available to fill, across every network</span>
						{dropped.length > 0 ? (
							<small>{`+${dropped.length} more ${dropped.map((cell) => cell.symbol).join(", ")}`}</small>
						) : null}
					</div>
					<div
						className="operator-balance-totals"
						style={{ "--balance-total-columns": Math.max(cells.length, 1) } as CSSProperties}
					>
						{cells.map((cell) => (
							<TokenTotalCell cell={cell} network={label} key={cell.symbol} />
						))}
						{cells.length === 0 ? (
							<span className="operator-balance-missing">No tracked tokens on any network</span>
						) : null}
					</div>

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
						{selected.assets.map((asset) => (
							<AssetBalanceCard asset={asset} key={asset.address} />
						))}
						{selected.assets.length === 0 ? (
							<span className="operator-balance-missing">No tracked tokens</span>
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

function TokenTotalCell(props: { cell: TokenTotal; network: string }) {
	const { cell, network } = props
	const share = cell.total !== null && cell.total > 0 ? ((cell.onSelected ?? 0) / cell.total) * 100 : 0
	const here = cell.onSelected !== null && cell.onSelected > 0

	return (
		<div className="operator-balance-total" data-token={cell.symbol.trim().toLowerCase()}>
			<span className="operator-balance-total-token">
				<TokenIcon symbol={cell.symbol} size="sm" />
				{cell.symbol}
			</span>
			<strong data-unavailable={cell.total === null ? true : undefined}>
				{cell.total === null ? "Unavailable" : formatAmount(cell.total)}
			</strong>
			{/* The bar floors at 2% so a sliver still reads; the caption below states the real share. */}
			<span className="operator-balance-share" data-unavailable={cell.total === null ? true : undefined}>
				<i
					style={{ width: cell.total === null ? "100%" : here ? `${Math.max(Math.round(share), 2)}%` : "0%" }}
				/>
			</span>
			<small data-muted={here ? undefined : true}>
				{cell.total === null
					? "a read failed — not estimated"
					: !here
						? `none on ${network}`
						: share < 1
							? `<1% of it on ${network}`
							: `${Math.round(share)}% of it on ${network}`}
			</small>
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

type TokenTotal = {
	symbol: string
	/** Available to fill on every network at once. Null the moment one contributor failed to read. */
	total: number | null
	/** How much of `total` sits on the selected network. Null whenever `total` is. */
	onSelected: number | null
}

/**
 * One entry per tracked symbol, in the order the config declares them. Deliberately not sorted
 * by magnitude: these are different assets, and 151,744 cNGN is not "more" than 98,144 USDC.
 */
function tokenTotals(chains: ChainBalances[], selectedChainId: number): TokenTotal[] {
	const totals = new Map<string, TokenTotal>()
	for (const chain of chains) {
		for (const asset of chain.assets) {
			const key = asset.symbol.trim().toUpperCase()
			const entry = totals.get(key) ?? { symbol: asset.symbol.trim(), total: 0, onSelected: 0 }
			totals.set(key, entry)
			if (asset.available === null) {
				entry.total = null
				entry.onSelected = null
				continue
			}
			if (entry.total === null) continue
			entry.total += asset.available
			if (chain.chainId === selectedChainId) entry.onSelected = (entry.onSelected ?? 0) + asset.available
		}
	}
	return [...totals.values()]
}

function chainLabel(status: StatusOperator, chainId: number): string {
	return status.chainLabels?.[String(chainId)] ?? `Chain ${chainId}`
}

/**
 * Available USDC and USDT across `assets`, or null when any contributing read failed. Refusing
 * to estimate is the point: a sum quietly missing a leg understates liquidity without saying so.
 */
export function availableStablecoins(assets: AssetBalance[], snapshot: SnapshotStatus): number | null {
	if (snapshot === "loading") return null
	const stables = assets.filter((asset) => STABLES.has(asset.symbol.trim().toUpperCase()))
	if (stables.length === 0) return snapshot === "fresh" ? 0 : null
	if (stables.some((asset) => asset.available === null)) return null
	return stables.reduce((total, asset) => total + (asset.available ?? 0), 0)
}

export function availableStablecoinLiquidity(balances: BalanceSnapshot | undefined): number | null {
	if (!balances) return null
	return availableStablecoins(
		balances.chains.flatMap((chain) => chain.assets),
		balances.status,
	)
}
