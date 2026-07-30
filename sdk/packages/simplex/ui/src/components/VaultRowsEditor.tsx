import { useState } from "react"
import { INIT_CHAINS } from "@/cli/init/chains"
import type { VaultRowDraft } from "../lib/vault-rows"
import type { KnownVault } from "../types"

const EXPLORER_BY_CHAIN = new Map(INIT_CHAINS.map((meta) => [meta.stateMachineId, meta.explorerUrl]))

/** Icon-only copy control with a transient ✓ acknowledgment. */
function CopyAddressButton(props: { address: string }) {
	const [copied, setCopied] = useState(false)
	const copy = async () => {
		try {
			await navigator.clipboard.writeText(props.address)
			setCopied(true)
			setTimeout(() => setCopied(false), 1200)
		} catch {
			// clipboard unavailable — nothing to acknowledge
		}
	}
	return (
		<button
			type="button"
			title="Copy vault address"
			onClick={copy}
			style={{ padding: "0 0.4rem", fontSize: "0.75rem", lineHeight: "1.4" }}
		>
			{copied ? "✓" : "⧉"}
		</button>
	)
}

export interface VaultChainOption {
	/** State machine id, e.g. "EVM-8453" — the value stored in the row's `chain`. */
	key: string
	label: string
}

const ROW_DEFAULTS = { threshold: "5000", minBalance: "3000", redeemOnShutdown: false }

/**
 * ERC-4626 vault selection, shared by the wizard Treasury step and the
 * operator Operations card. Registry vaults are ticked on and off (with a
 * select-all); ticking one unfolds its threshold/min-balance settings right
 * under the name. Custom vaults are typed in as free-form rows below.
 */
export function VaultRowsEditor(props: {
	chains: VaultChainOption[]
	knownVaults: Record<string, KnownVault[]>
	rows: VaultRowDraft[]
	onChange: (rows: VaultRowDraft[]) => void
}) {
	const { chains, knownVaults, rows, onChange } = props

	const catalog = chains.flatMap((chain) => (knownVaults[chain.key] ?? []).map((known) => ({ chain, known })))
	const rowIndexOf = (chainKey: string, address: string) =>
		rows.findIndex((r) => r.chain === chainKey && r.vault.toLowerCase() === address.toLowerCase())
	const withoutRow = (list: VaultRowDraft[], chainKey: string, address: string) =>
		list.filter((r) => !(r.chain === chainKey && r.vault.toLowerCase() === address.toLowerCase()))
	const knownFor = (row: VaultRowDraft): KnownVault | undefined =>
		(knownVaults[row.chain] ?? []).find((k) => k.address.toLowerCase() === row.vault.trim().toLowerCase())
	const patch = (index: number, changes: Partial<VaultRowDraft>) =>
		onChange(rows.map((r, i) => (i === index ? { ...r, ...changes } : r)))

	const allSelected = catalog.length > 0 && catalog.every(({ chain, known }) => rowIndexOf(chain.key, known.address) >= 0)

	const settingsFields = (index: number, row: VaultRowDraft) => (
		<>
			<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
				<span>Sweep threshold ($)</span>
				<input type="text" value={row.threshold} onChange={(e) => patch(index, { threshold: e.target.value })} />
			</label>
			<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
				<span>Min balance ($)</span>
				<input type="text" value={row.minBalance} onChange={(e) => patch(index, { minBalance: e.target.value })} />
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
		</>
	)

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
													.filter(({ chain, known }) => rowIndexOf(chain.key, known.address) < 0)
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
					{catalog.map(({ chain, known }) => {
						const index = rowIndexOf(chain.key, known.address)
						const explorer = EXPLORER_BY_CHAIN.get(chain.key)
						return (
							<div key={`${chain.key}-${known.address}`}>
								<div className="row">
									{/* Buttons live outside the label so clicking them never toggles the checkbox. */}
									<label className="row" style={{ margin: 0 }}>
										<input
											type="checkbox"
											checked={index >= 0}
											onChange={(e) =>
												onChange(
													e.target.checked
														? [...rows, { chain: chain.key, vault: known.address, ...ROW_DEFAULTS }]
														: withoutRow(rows, chain.key, known.address),
												)
											}
										/>
										{chain.label}: {known.label} ({known.asset})
									</label>
									<CopyAddressButton address={known.address} />
									{explorer && (
										<button
											type="button"
											title="Open the vault address on the block explorer"
											onClick={() =>
												window.open(`${explorer}/address/${known.address}`, "_blank", "noopener")
											}
											style={{ padding: "0 0.4rem", fontSize: "0.75rem", lineHeight: "1.4" }}
										>
											↗
										</button>
									)}
								</div>
								{index >= 0 && (
									<div
										className="row"
										style={{ margin: "0.2rem 0 0.7rem 1.7rem", alignItems: "flex-end" }}
									>
										{settingsFields(index, rows[index])}
									</div>
								)}
							</div>
						)
					})}
				</div>
			)}

			{rows.map((row, index) =>
				knownFor(row) ? null : (
					// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
					<div className="row" key={index} style={{ margin: "0.4rem 0", alignItems: "flex-end" }}>
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
						{settingsFields(index, row)}
						<button type="button" onClick={() => onChange(rows.filter((_, i) => i !== index))}>
							✕
						</button>
					</div>
				),
			)}
			<button
				type="button"
				onClick={() => onChange([...rows, { chain: chains[0]?.key ?? "", vault: "", ...ROW_DEFAULTS }])}
			>
				+ Add vault
			</button>
		</div>
	)
}
