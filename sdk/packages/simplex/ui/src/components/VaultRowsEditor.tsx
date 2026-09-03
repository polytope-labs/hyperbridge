import * as Collapsible from "@radix-ui/react-collapsible"
import { Tooltip, TooltipContent, TooltipTrigger } from "@hyperbridge/ui"
import { CircleInfo } from "@hyperbridge/ui/icons"
import { useId, useState } from "react"
import { INIT_CHAINS } from "@/cli/init/chains"
import type { VaultRowDraft } from "../lib/vault-rows"
import type { KnownVault } from "../types"
import { ChainLogo } from "./ChainLogo"
import { CheckIcon, CloseIcon, CopyIcon, ExternalLinkIcon } from "./InterfaceIcons"
import { TokenIcon } from "./TokenIcon"
import { WizardDialog } from "./WizardDialog"

const EXPLORER_BY_CHAIN = new Map(INIT_CHAINS.map((meta) => [meta.stateMachineId, meta.explorerUrl]))
const ROW_DEFAULTS = { threshold: "5000", minBalance: "3000", redeemOnShutdown: false }
const CURATED_VAULT_DEFAULTS = new Map([
	["Aave stataUSDC", { threshold: "20", minBalance: "10", redeemOnShutdown: false }],
	["Yield Bearing cNGN", { threshold: "1000", minBalance: "1", redeemOnShutdown: false }],
])

const VAULT_FIELD_HELP = {
	threshold:
		"When the wallet balance reaches this USD value, Simplex sweeps the funds above the minimum wallet balance into this vault.",
	minBalance:
		"The USD value Simplex keeps in the wallet after a sweep so liquidity remains immediately available for fills.",
} as const

export interface VaultChainOption {
	/** State machine id, e.g. "EVM-8453" — the value stored in the row's `chain`. */
	key: string
	label: string
}

type CustomVaultDraft = VaultRowDraft & { index: number | null }

function withoutVaultRow(list: VaultRowDraft[], chainKey: string, address: string) {
	return list.filter((row) => !(row.chain === chainKey && row.vault.toLowerCase() === address.toLowerCase()))
}

function CopyAddressButton({ address }: { address: string }) {
	const [copied, setCopied] = useState(false)
	const copy = () => {
		void navigator.clipboard
			.writeText(address)
			.then(() => {
				setCopied(true)
				window.setTimeout(() => setCopied(false), 1200)
			})
			.catch(() => setCopied(false))
	}

	return (
		<button
			type="button"
			className="vault-icon-action"
			aria-label={copied ? "Vault address copied" : "Copy vault address"}
			title={copied ? "Copied" : "Copy vault address"}
			onClick={copy}
		>
			{copied ? <CheckIcon aria-hidden="true" /> : <CopyIcon aria-hidden="true" />}
		</button>
	)
}

function defaultsForKnownVault(known: KnownVault) {
	return CURATED_VAULT_DEFAULTS.get(known.label) ?? ROW_DEFAULTS
}

function VaultAmountField(props: {
	label: string
	help: string
	value: string
	onChange: (value: string) => void
}) {
	const { label, help, value, onChange } = props
	const inputId = useId()

	return (
		<div className="field vault-amount-field">
			<div className="field-label vault-field-label">
				<label htmlFor={inputId}>{label}</label>
				<Tooltip>
					<TooltipTrigger asChild>
						<button type="button" className="vault-info-trigger" aria-label={`About ${label}`}>
							<CircleInfo aria-hidden="true" />
						</button>
					</TooltipTrigger>
					<TooltipContent className="simplex-tooltip-content" sideOffset={7}>
						<p>{help}</p>
					</TooltipContent>
				</Tooltip>
			</div>
			<input id={inputId} type="text" inputMode="decimal" value={value} onChange={(event) => onChange(event.target.value)} />
		</div>
	)
}

function VaultSettings(props: { row: VaultRowDraft; onChange: (changes: Partial<VaultRowDraft>) => void }) {
	const { row, onChange } = props
	return (
		<div className="vault-settings-grid">
			<VaultAmountField
				label="Sweep threshold (USD)"
				help={VAULT_FIELD_HELP.threshold}
				value={row.threshold}
				onChange={(threshold) => onChange({ threshold })}
			/>
			<VaultAmountField
				label="Minimum wallet balance (USD)"
				help={VAULT_FIELD_HELP.minBalance}
				value={row.minBalance}
				onChange={(minBalance) => onChange({ minBalance })}
			/>
			<label className="vault-redeem-toggle">
				<input
					type="checkbox"
					checked={row.redeemOnShutdown}
					onChange={(event) => onChange({ redeemOnShutdown: event.target.checked })}
				/>
				<span>
					<strong>Redeem on shutdown</strong>
					<small>Return this position to the wallet before Simplex stops.</small>
				</span>
			</label>
		</div>
	)
}

function CuratedVaultRow(props: {
	chain: VaultChainOption
	known: KnownVault
	row: VaultRowDraft | undefined
	onToggle: (selected: boolean) => void
	onChange: (changes: Partial<VaultRowDraft>) => void
}) {
	const { chain, known, row, onToggle, onChange } = props
	const selected = row !== undefined
	const explorer = EXPLORER_BY_CHAIN.get(chain.key)

	return (
		<Collapsible.Root
			className="vault-item"
			data-selected={selected}
			open={selected}
			key={`${chain.key}-${known.address}`}
		>
			<div className="vault-row">
				<label className="vault-selection">
					<input type="checkbox" checked={selected} onChange={(event) => onToggle(event.target.checked)} />
					<span className="vault-selection-indicator" aria-hidden="true">
						<CheckIcon />
					</span>
					<span className="vault-logo-stack" aria-hidden="true">
						<ChainLogo label={chain.label} />
						<TokenIcon symbol={known.asset} />
					</span>
					<span className="vault-row-copy">
						<strong>{known.label}</strong>
						<small>
							{chain.label} · {known.asset}
						</small>
					</span>
				</label>
				<div className="vault-row-actions">
					<CopyAddressButton address={known.address} />
					{explorer && (
						<a
							className="vault-icon-action"
							href={`${explorer}/address/${known.address}`}
							target="_blank"
							rel="noreferrer"
							aria-label={`Open ${known.label} on the block explorer`}
							title="Open on block explorer"
						>
							<ExternalLinkIcon aria-hidden="true" />
						</a>
					)}
				</div>
			</div>
			<Collapsible.Content className="vault-settings-collapsible">
				{row && <VaultSettings row={row} onChange={onChange} />}
			</Collapsible.Content>
		</Collapsible.Root>
	)
}

function CustomVaultRow(props: {
	row: VaultRowDraft
	chainLabel: string
	onConfigure: () => void
	onRemove: () => void
}) {
	const { row, chainLabel, onConfigure, onRemove } = props
	return (
		<div className="vault-item vault-custom-row" data-selected="true">
			<div className="vault-row">
				<div className="vault-selection vault-custom-identity">
					<span className="vault-logo-stack" aria-hidden="true">
						<ChainLogo label={chainLabel} />
						<TokenIcon symbol="" />
					</span>
					<span className="vault-row-copy">
						<strong>Custom ERC-4626 vault</strong>
						<small>
							{chainLabel} · {row.vault.slice(0, 10)}…
						</small>
					</span>
				</div>
				<div className="vault-row-actions vault-custom-actions">
					<button type="button" className="market-configure-button" onClick={onConfigure}>
						Configure
					</button>
					<button
						type="button"
						className="vault-icon-action"
						aria-label="Remove custom vault"
						onClick={onRemove}
					>
						<CloseIcon aria-hidden="true" />
					</button>
				</div>
			</div>
		</div>
	)
}

function CustomVaultDialog(props: {
	draft: CustomVaultDraft | undefined
	chains: VaultChainOption[]
	onChange: (draft: CustomVaultDraft) => void
	onClose: () => void
	onSave: () => void
}) {
	const { draft, chains, onChange, onClose, onSave } = props
	return (
		<WizardDialog
			open={draft !== undefined}
			onClose={onClose}
			title={draft?.index === null ? "Add a custom vault" : "Configure custom vault"}
			description="Connect one ERC-4626 vault for an asset on this chain."
		>
			{draft && (
				<div className="vault-custom-modal">
					<div className="vault-custom-primary-fields">
						<label className="field">
							<span>Chain</span>
							<select
								value={draft.chain}
								onChange={(event) => onChange({ ...draft, chain: event.target.value })}
							>
								{chains.map((chain) => (
									<option key={chain.key} value={chain.key}>
										{chain.label}
									</option>
								))}
							</select>
						</label>
						<label className="field">
							<span className="field-label field-label-required-mark">
								Vault address{" "}
								<span className="field-required-mark" aria-hidden="true">
									*
								</span>
							</span>
							<input
								type="text"
								required
								placeholder="0x…"
								value={draft.vault}
								onChange={(event) => onChange({ ...draft, vault: event.target.value })}
							/>
						</label>
					</div>
					<div className="vault-custom-balance-fields">
						<VaultAmountField
							label="Sweep threshold (USD)"
							help={VAULT_FIELD_HELP.threshold}
							value={draft.threshold}
							onChange={(threshold) => onChange({ ...draft, threshold })}
						/>
						<VaultAmountField
							label="Minimum wallet balance (USD)"
							help={VAULT_FIELD_HELP.minBalance}
							value={draft.minBalance}
							onChange={(minBalance) => onChange({ ...draft, minBalance })}
						/>
					</div>
					<label className="vault-redeem-toggle vault-custom-redeem">
						<input
							type="checkbox"
							checked={draft.redeemOnShutdown}
							onChange={(event) => onChange({ ...draft, redeemOnShutdown: event.target.checked })}
						/>
						<span>
							<strong>Redeem on shutdown</strong>
							<small>Return the position to the wallet before Simplex stops.</small>
						</span>
					</label>
					<footer className="market-dialog-footer market-dialog-footer-end">
						<button
							type="button"
							className="primary"
							disabled={!draft.chain || !draft.vault.trim()}
							onClick={onSave}
						>
							Save vault
						</button>
					</footer>
				</div>
			)}
		</WizardDialog>
	)
}

export function VaultRowsEditor(props: {
	chains: VaultChainOption[]
	knownVaults: Record<string, KnownVault[]>
	rows: VaultRowDraft[]
	onChange: (rows: VaultRowDraft[]) => void
}) {
	const { chains, knownVaults, rows, onChange } = props
	const [customDraft, setCustomDraft] = useState<CustomVaultDraft>()
	const catalog = chains.flatMap((chain) => (knownVaults[chain.key] ?? []).map((known) => ({ chain, known })))
	const chainLabelByKey = new Map(chains.map((chain) => [chain.key, chain.label]))

	const rowIndexOf = (chainKey: string, address: string) =>
		rows.findIndex((row) => row.chain === chainKey && row.vault.toLowerCase() === address.toLowerCase())
	const knownFor = (row: VaultRowDraft): KnownVault | undefined =>
		(knownVaults[row.chain] ?? []).find((known) => known.address.toLowerCase() === row.vault.trim().toLowerCase())
	const patch = (index: number, changes: Partial<VaultRowDraft>) =>
		onChange(rows.map((row, rowIndex) => (rowIndex === index ? { ...row, ...changes } : row)))

	const allSelected =
		catalog.length > 0 && catalog.every(({ chain, known }) => rowIndexOf(chain.key, known.address) >= 0)
	const customRows = rows.reduce<Array<{ row: VaultRowDraft; index: number }>>((list, row, index) => {
		if (!knownFor(row)) list.push({ row, index })
		return list
	}, [])
	const unselectedCatalogRows = catalog.reduce<VaultRowDraft[]>((list, { chain, known }) => {
		if (rowIndexOf(chain.key, known.address) < 0) {
			list.push({ chain: chain.key, vault: known.address, ...defaultsForKnownVault(known) })
		}
		return list
	}, [])

	const toggleAll = () =>
		onChange(
			allSelected
				? catalog.reduce((list, { chain, known }) => withoutVaultRow(list, chain.key, known.address), rows)
				: [...rows, ...unselectedCatalogRows],
		)

	const openNewCustomVault = () =>
		setCustomDraft({ index: null, chain: chains[0]?.key ?? "", vault: "", ...ROW_DEFAULTS })

	const saveCustomVault = () => {
		if (!customDraft?.chain || !customDraft.vault.trim()) return
		const next = { ...customDraft, vault: customDraft.vault.trim() }
		const { index, ...row } = next
		onChange(
			index === null ? [...rows, row] : rows.map((current, rowIndex) => (rowIndex === index ? row : current)),
		)
		setCustomDraft(undefined)
	}

	return (
		<div className="vault-editor">
			<div className="vault-editor-heading">
				<div>
					<h3>Available vaults</h3>
					<p>Choose a curated vault, then set how much liquidity stays in the wallet.</p>
				</div>
				<div className="vault-editor-actions">
					{catalog.length > 1 && (
						<button type="button" className="market-text-action" onClick={toggleAll}>
							{allSelected ? "Clear curated vaults" : "Select all"}
						</button>
					)}
					<button type="button" className="market-create-button" onClick={openNewCustomVault}>
						<span aria-hidden="true">+</span> Custom vault
					</button>
				</div>
			</div>

			<div className="vault-list">
				{catalog.map(({ chain, known }) => {
					const index = rowIndexOf(chain.key, known.address)
					return (
						<CuratedVaultRow
							key={`${chain.key}-${known.address}`}
							chain={chain}
							known={known}
							row={index >= 0 ? rows[index] : undefined}
							onToggle={(selected) =>
								onChange(
									selected
										? [...rows, { chain: chain.key, vault: known.address, ...defaultsForKnownVault(known) }]
										: withoutVaultRow(rows, chain.key, known.address),
								)
							}
							onChange={(changes) => patch(index, changes)}
						/>
					)
				})}

				{customRows.map(({ row, index }) => {
					const chainLabel = chainLabelByKey.get(row.chain) ?? row.chain
					return (
						<CustomVaultRow
							key={`${row.chain}-${row.vault}-${index}`}
							row={row}
							chainLabel={chainLabel}
							onConfigure={() => setCustomDraft({ ...row, index })}
							onRemove={() => onChange(rows.filter((_, rowIndex) => rowIndex !== index))}
						/>
					)
				})}

				{catalog.length === 0 && customRows.length === 0 && (
					<div className="vault-empty-state">
						<strong>No curated vaults are available for the enabled chains.</strong>
						<p>Add a custom ERC-4626 address if you still want treasury automation.</p>
					</div>
				)}
			</div>

			<CustomVaultDialog
				draft={customDraft}
				chains={chains}
				onChange={setCustomDraft}
				onClose={() => setCustomDraft(undefined)}
				onSave={saveCustomVault}
			/>
		</div>
	)
}
