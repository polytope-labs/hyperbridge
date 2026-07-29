import type { VaultRowDraft } from "../lib/vault-rows"
import type { KnownVault } from "../types"

export interface VaultChainOption {
	/** State machine id, e.g. "EVM-8453" — the value stored in the row's `chain`. */
	key: string
	label: string
}

const ROW_DEFAULTS = { threshold: "5000", minBalance: "3000", redeemOnShutdown: false }

/**
 * ERC-4626 vault selection, shared by the wizard Treasury step and the
 * operator Operations card: registry vaults are ticked on and off (with a
 * select-all), custom vaults are typed in as free-form rows.
 */
export function VaultRowsEditor(props: {
	chains: VaultChainOption[]
	knownVaults: Record<string, KnownVault[]>
	rows: VaultRowDraft[]
	onChange: (rows: VaultRowDraft[]) => void
}) {
	const { chains, knownVaults, rows, onChange } = props

	const catalog = chains.flatMap((chain) => (knownVaults[chain.key] ?? []).map((known) => ({ chain, known })))
	const hasRow = (chainKey: string, address: string) =>
		rows.some((r) => r.chain === chainKey && r.vault.toLowerCase() === address.toLowerCase())
	const withoutRow = (list: VaultRowDraft[], chainKey: string, address: string) =>
		list.filter((r) => !(r.chain === chainKey && r.vault.toLowerCase() === address.toLowerCase()))
	const knownFor = (row: VaultRowDraft): KnownVault | undefined =>
		(knownVaults[row.chain] ?? []).find((k) => k.address.toLowerCase() === row.vault.trim().toLowerCase())
	const chainLabel = (key: string) => chains.find((c) => c.key === key)?.label ?? key
	const patch = (index: number, changes: Partial<VaultRowDraft>) =>
		onChange(rows.map((r, i) => (i === index ? { ...r, ...changes } : r)))

	const allSelected = catalog.length > 0 && catalog.every(({ chain, known }) => hasRow(chain.key, known.address))

	return (
		<div>
			{catalog.length > 0 && (
				<div style={{ marginBottom: "0.8rem" }}>
					<p className="hint">Known vaults — tick to add:</p>
					<label className="row">
						<input
							type="checkbox"
							checked={allSelected}
							onChange={(e) =>
								onChange(
									e.target.checked
										? [
												...rows,
												...catalog
													.filter(({ chain, known }) => !hasRow(chain.key, known.address))
													.map(({ chain, known }) => ({
														chain: chain.key,
														vault: known.address,
														...ROW_DEFAULTS,
													})),
											]
										: catalog.reduce(
												(list, { chain, known }) => withoutRow(list, chain.key, known.address),
												rows,
											),
								)
							}
						/>
						Select all
					</label>
					{catalog.map(({ chain, known }) => (
						<label className="row" key={`${chain.key}-${known.address}`}>
							<input
								type="checkbox"
								checked={hasRow(chain.key, known.address)}
								onChange={(e) =>
									onChange(
										e.target.checked
											? [...rows, { chain: chain.key, vault: known.address, ...ROW_DEFAULTS }]
											: withoutRow(rows, chain.key, known.address),
									)
								}
							/>
							{chain.label}: {known.label} ({known.asset}){" "}
							<span className="mono">{known.address.slice(0, 10)}…</span>
						</label>
					))}
				</div>
			)}

			{rows.map((row, index) => {
				const known = knownFor(row)
				return (
					// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
					<div className="row" key={index} style={{ margin: "0.4rem 0", alignItems: "flex-end" }}>
						{known ? (
							<span style={{ flex: 1, whiteSpace: "nowrap" }}>
								{chainLabel(row.chain)}: {known.label} ({known.asset}){" "}
								<span className="mono">{row.vault.slice(0, 10)}…</span>
							</span>
						) : (
							<>
								<label className="field" style={{ margin: 0 }}>
									<span>Chain</span>
									<select value={row.chain} onChange={(e) => patch(index, { chain: e.target.value })}>
										{chains.map((c) => (
											<option key={c.key} value={c.key}>
												{c.label}
											</option>
										))}
									</select>
								</label>
								<label className="field" style={{ flex: 1, margin: 0 }}>
									<span>Vault address</span>
									<input
										type="text"
										placeholder="0x…"
										value={row.vault}
										onChange={(e) => patch(index, { vault: e.target.value })}
									/>
								</label>
							</>
						)}
						<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
							<span>Sweep threshold ($)</span>
							<input
								type="text"
								value={row.threshold}
								onChange={(e) => patch(index, { threshold: e.target.value })}
							/>
						</label>
						<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
							<span>Min balance ($)</span>
							<input
								type="text"
								value={row.minBalance}
								onChange={(e) => patch(index, { minBalance: e.target.value })}
							/>
						</label>
						<label
							className="row"
							style={{ whiteSpace: "nowrap" }}
							title="Redeem this position to the wallet on graceful shutdown"
						>
							<input
								type="checkbox"
								checked={row.redeemOnShutdown}
								onChange={(e) => patch(index, { redeemOnShutdown: e.target.checked })}
							/>
							redeem on shutdown
						</label>
						<button type="button" onClick={() => onChange(rows.filter((_, i) => i !== index))}>
							✕
						</button>
					</div>
				)
			})}
			<button
				type="button"
				onClick={() => onChange([...rows, { chain: chains[0]?.key ?? "", vault: "", ...ROW_DEFAULTS }])}
			>
				+ Add vault
			</button>
		</div>
	)
}
