import type { RebalancingDto, RebalancingHub } from "../types"
import { SelectField } from "./AppSelect"
import { CheckIcon } from "./InterfaceIcons"
import { TokenIcon } from "./TokenIcon"
import { VaultAmountField } from "./VaultRowsEditor"

type AssetOptions = RebalancingDto["assets"][number]
type ChainCoverage = AssetOptions["hubs"][number]["chains"][number]

const HUB_DEFAULTS = { threshold: "6000", minBalance: "5000" }

const ROUTE_LABELS: Record<NonNullable<ChainCoverage["method"]>, string> = {
	cctp: "CCTP",
	usdt0: "USDT0",
	cex: "Binance",
}

/** Why a chain is left out of a hub, as the end of "BNB Chain: …". */
function uncoveredReason(entry: ChainCoverage, asset: string, hubLabel: string): string {
	if (entry.uncovered === "no-asset") return `no ${asset} there`
	if (entry.uncovered === "vault") return `its ${asset} goes to a vault`
	return `no route to ${hubLabel}`
}

/** Which chains a hub gathers from, and every chain it leaves out with the reason. */
function HubCoverage(props: {
	asset: string
	hubLabel: string
	chains: ChainCoverage[]
	chainLabel: (chain: string) => string
}) {
	const { asset, hubLabel, chains, chainLabel } = props
	const covered = chains.filter((entry) => entry.method)
	const uncovered = chains.filter((entry) => !entry.method)
	return (
		<div className="rebalancing-coverage">
			<p className={covered.length > 0 ? "hint" : "warning"}>
				{covered.length > 0
					? `Gathers from ${covered
							.map((entry) => `${chainLabel(entry.chain)} (${ROUTE_LABELS[entry.method!]})`)
							.join(", ")}.`
					: `No other chain can bridge ${asset} to ${hubLabel}, so nothing will move.`}
			</p>
			{uncovered.length > 0 && (
				<p className="hint">
					Left out:{" "}
					{uncovered
						.map((entry) => `${chainLabel(entry.chain)} (${uncoveredReason(entry, asset, hubLabel)})`)
						.join(", ")}
					.
				</p>
			)}
		</div>
	)
}

function HubRow(props: {
	options: AssetOptions
	row: RebalancingHub | undefined
	chainLabel: (chain: string) => string
	onToggle: (selected: boolean) => void
	onChange: (changes: Partial<RebalancingHub>) => void
}) {
	const { options, row, chainLabel, onToggle, onChange } = props
	const { asset } = options
	const selected = row !== undefined
	// A saved hub stays editable even when nothing could be its hub any more, so it can be removed.
	const locked = options.hubs.length === 0 && !selected
	const hub = row && options.hubs.find((candidate) => candidate.chain === row.chain)

	return (
		<div className="vault-item" data-selected={selected} data-disabled={locked}>
			<div className="vault-row">
				<label
					className="vault-selection"
					title={locked ? `No enabled chain has ${asset} in the asset registry` : undefined}
				>
					<input
						type="checkbox"
						checked={selected}
						disabled={locked}
						onChange={(event) => onToggle(event.target.checked)}
					/>
					<span className="vault-selection-indicator" aria-hidden="true">
						<CheckIcon />
					</span>
					<span className="vault-logo-token" aria-hidden="true">
						<TokenIcon symbol={asset} />
					</span>
					<span className="vault-row-copy">
						<strong>{asset}</strong>
						<small>{row ? `Gathers on ${chainLabel(row.chain)}` : "Not rebalanced"}</small>
					</span>
				</label>
			</div>
			{row && (
				<>
					<div className="vault-settings-grid">
						<SelectField
							label="Hub chain"
							value={row.chain}
							options={[
								...options.hubs.map((candidate) => ({
									value: candidate.chain,
									label: chainLabel(candidate.chain),
								})),
								// A saved hub that is no longer a choice still has to show as the value.
								...(hub ? [] : [{ value: row.chain, label: chainLabel(row.chain) }]),
							]}
							onValueChange={(chain) => onChange({ chain })}
						/>
						<VaultAmountField
							label={`Bridge threshold (${asset})`}
							help={`When a chain's wallet reaches this ${asset} amount, Simplex bridges everything above the minimum wallet balance to the hub.`}
							value={row.threshold}
							onChange={(threshold) => onChange({ threshold })}
						/>
						<VaultAmountField
							label={`Minimum wallet balance (${asset})`}
							help={`Simplex never bridges below this amount. It stays in each chain's wallet for fills on that chain. The gap up to the threshold is the smallest transfer.`}
							value={row.minBalance}
							onChange={(minBalance) => onChange({ minBalance })}
						/>
					</div>
					{hub ? (
						<HubCoverage
							asset={asset}
							hubLabel={chainLabel(hub.chain)}
							chains={hub.chains}
							chainLabel={chainLabel}
						/>
					) : (
						<div className="rebalancing-coverage">
							<p className="warning">
								{chainLabel(row.chain)} cannot be the hub for {asset}. Pick another chain or clear this
								asset.
							</p>
						</div>
					)}
				</>
			)}
		</div>
	)
}

/**
 * One row per asset a route can bridge: whether it is rebalanced, its hub chain, the
 * threshold that triggers a transfer and the balance every other chain keeps.
 */
export function RebalancingHubsEditor(props: {
	/** Every asset that can be rebalanced, with its possible hubs and what each covers. */
	assets: RebalancingDto["assets"]
	/** Display name for a state machine id such as "EVM-8453". */
	chainLabel: (chain: string) => string
	rows: RebalancingHub[]
	onChange: (rows: RebalancingHub[]) => void
}) {
	const { assets, chainLabel, rows, onChange } = props
	const rowFor = (asset: string) => rows.find((row) => row.asset.toUpperCase() === asset)
	// A saved hub for an asset no route carries any more still gets a row, so it can be cleared.
	const orphans = rows
		.filter((row) => !assets.some((options) => options.asset === row.asset.toUpperCase()))
		.map((row) => ({ asset: row.asset.toUpperCase(), hubs: [] }))

	return (
		<div className="vault-editor">
			<div className="vault-editor-heading">
				<div>
					<h3>Hub per asset</h3>
					<p>
						Pick the chain each asset gathers on. Every other enabled chain bridges its excess there on its
						own.
					</p>
				</div>
			</div>
			<div className="vault-groups">
				<section className="vault-group" aria-label="Rebalanced assets">
					{[...assets, ...orphans].map((options) => (
						<HubRow
							key={options.asset}
							options={options}
							row={rowFor(options.asset)}
							chainLabel={chainLabel}
							onToggle={(selected) =>
								onChange(
									selected
										? [
												...rows,
												{ asset: options.asset, chain: options.hubs[0].chain, ...HUB_DEFAULTS },
											]
										: rows.filter((row) => row.asset.toUpperCase() !== options.asset),
								)
							}
							onChange={(changes) =>
								onChange(
									rows.map((row) =>
										row.asset.toUpperCase() === options.asset ? { ...row, ...changes } : row,
									),
								)
							}
						/>
					))}
				</section>
			</div>
		</div>
	)
}
