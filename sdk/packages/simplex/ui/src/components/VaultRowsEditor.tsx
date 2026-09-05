import * as Collapsible from "@radix-ui/react-collapsible"
import { Tooltip, TooltipContent, TooltipTrigger } from "@hyperbridge/ui"
import { CircleInfo } from "@hyperbridge/ui/icons"
import { type ReactNode, useId, useState } from "react"
import { INIT_CHAINS, type InitNetwork } from "@/cli/init/chains"
import type { VaultRowDraft } from "../lib/vault-rows"
import type { KnownVault } from "../types"
import { ChainLogo } from "./ChainLogo"
import { CheckIcon, ChevronDownIcon, ChevronRightIcon, CloseIcon, CopyIcon, ExternalLinkIcon } from "./InterfaceIcons"
import { TokenIcon } from "./TokenIcon"
import { WizardDialog } from "./WizardDialog"

const EXPLORER_BY_CHAIN = new Map(INIT_CHAINS.map((meta) => [meta.stateMachineId, meta.explorerUrl]))
const ROW_DEFAULTS = { threshold: "5000", minBalance: "3000", redeemOnShutdown: false }
const CURATED_VAULT_DEFAULTS = new Map([
	["Aave stataUSDC", { threshold: "20", minBalance: "10", redeemOnShutdown: false }],
	["Yield Bearing cNGN", { threshold: "1000", minBalance: "1", redeemOnShutdown: false }],
])

function vaultFieldLabel(field: "threshold" | "minBalance", asset: string) {
	return `${field === "threshold" ? "Sweep threshold" : "Minimum wallet balance"} (${asset})`
}

function vaultFieldHelp(field: "threshold" | "minBalance", asset: string) {
	if (field === "threshold") {
		return `When the wallet balance reaches this ${asset} amount, Simplex sweeps the funds above the minimum wallet balance into this vault.`
	}
	// Fills draw from the vault position, so the wallet float is not fill liquidity; USDC and
	// USDT are singled out because they are the tokens the paymaster charges gas in.
	const gasNote = ["USDC", "USDT"].includes(asset)
		? ` ${asset} also pays for gas through the paymaster, so keep enough here to cover that.`
		: ""
	return `Simplex never sweeps below this amount. It stays in the wallet as ${asset} you can spend at any time; everything above it goes into the vault.${gasNote}`
}

export interface VaultChainOption {
	/** State machine id, e.g. "EVM-8453" — the value stored in the row's `chain`. */
	key: string
	label: string
}

/** One chain's slice of the catalog: its curated vaults plus whether a vault can be saved for it. */
interface ChainGroup extends VaultChainOption {
	enabled: boolean
	vaults: KnownVault[]
}

/**
 * Enabled chains first (in the order given), then every other chain the catalog
 * has vaults for, in the initialization catalog's order. Chains outside `network`
 * are dropped so the wizard never shows testnet vaults on a mainnet draft.
 */
function chainGroups(
	enabled: VaultChainOption[],
	knownVaults: Record<string, KnownVault[]>,
	network: InitNetwork | undefined,
): ChainGroup[] {
	const enabledKeys = new Set(enabled.map((chain) => chain.key))
	const others = INIT_CHAINS.flatMap((meta) => {
		if (enabledKeys.has(meta.stateMachineId)) return []
		if (network && meta.network !== network) return []
		const vaults = knownVaults[meta.stateMachineId] ?? []
		if (vaults.length === 0) return []
		return [{ key: meta.stateMachineId, label: meta.label, enabled: false, vaults }]
	})
	return [...enabled.map((chain) => ({ ...chain, enabled: true, vaults: knownVaults[chain.key] ?? [] })), ...others]
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

function VaultSettings(props: {
	row: VaultRowDraft
	asset: string
	onChange: (changes: Partial<VaultRowDraft>) => void
}) {
	const { row, asset, onChange } = props
	return (
		<div className="vault-settings-grid">
			<VaultAmountField
				label={vaultFieldLabel("threshold", asset)}
				help={vaultFieldHelp("threshold", asset)}
				value={row.threshold}
				onChange={(threshold) => onChange({ threshold })}
			/>
			<VaultAmountField
				label={vaultFieldLabel("minBalance", asset)}
				help={vaultFieldHelp("minBalance", asset)}
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
	/** False when the vault's chain is not enabled: shown for discovery, but not selectable. */
	enabled: boolean
	onToggle: (selected: boolean) => void
	onChange: (changes: Partial<VaultRowDraft>) => void
}) {
	const { chain, known, row, enabled, onToggle, onChange } = props
	const selected = row !== undefined
	// A row already in the config stays editable even if its chain was disabled
	// since, so the operator can still deselect it.
	const locked = !enabled && !selected
	const explorer = EXPLORER_BY_CHAIN.get(chain.key)

	return (
		<Collapsible.Root
			className="vault-item"
			data-selected={selected}
			data-disabled={locked}
			open={selected}
			key={`${chain.key}-${known.address}`}
		>
			<div className="vault-row">
				<label className="vault-selection" title={locked ? `Enable ${chain.label} first` : undefined}>
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
						<TokenIcon symbol={known.asset} />
					</span>
					<span className="vault-row-copy">
						<strong>{known.label}</strong>
						<small>{known.asset}</small>
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
				{row && <VaultSettings row={row} asset={known.asset} onChange={onChange} />}
			</Collapsible.Content>
		</Collapsible.Root>
	)
}

function CustomVaultRow(props: { row: VaultRowDraft; onConfigure: () => void; onRemove: () => void }) {
	const { row, onConfigure, onRemove } = props
	return (
		<div className="vault-item vault-custom-row" data-selected="true">
			<div className="vault-row">
				<div className="vault-selection vault-custom-identity">
					<span className="vault-logo-token" aria-hidden="true">
						<TokenIcon symbol="" />
					</span>
					<span className="vault-row-copy">
						<strong>Custom ERC-4626 vault</strong>
						<small>{row.vault.slice(0, 10)}…</small>
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

/**
 * One chain's vaults under a header that carries the chain identity and its
 * enabled state, so the rows themselves stay compact. A locked group explains
 * itself once, in the header, instead of on every row.
 */
function VaultChainGroup(props: {
	group: ChainGroup
	rows: VaultRowDraft[]
	customRows: Array<{ row: VaultRowDraft; index: number }>
	onEnableChain?: (chain: VaultChainOption) => void
	onToggle: (known: KnownVault, selected: boolean) => void
	onChange: (index: number, changes: Partial<VaultRowDraft>) => void
	onConfigureCustom: (row: VaultRowDraft, index: number) => void
	onRemoveCustom: (index: number) => void
}) {
	const { group, rows, customRows, onEnableChain, onToggle, onChange, onConfigureCustom, onRemoveCustom } = props
	const rowIndexOf = (address: string) =>
		rows.findIndex((row) => row.chain === group.key && row.vault.toLowerCase() === address.toLowerCase())
	const vaultCount = group.vaults.length + customRows.length

	return (
		<section className="vault-group" data-locked={!group.enabled} aria-label={group.label}>
			<header className="vault-group-head">
				<div className="vault-group-identity">
					<ChainLogo label={group.label} />
					<span>
						<strong>{group.label}</strong>
						<small>
							{vaultCount} {vaultCount === 1 ? "vault" : "vaults"}
							{customRows.length > 0 && ` · ${customRows.length} custom`}
						</small>
					</span>
				</div>
				<div className="vault-group-status">
					{group.enabled ? (
						<span className="vault-status-pill" data-tone="ok">
							<span className="vault-status-dot" aria-hidden="true" />
							Enabled
						</span>
					) : (
						<>
							<span className="vault-status-pill">Not enabled</span>
							{onEnableChain && (
								<button type="button" className="vault-enable-link" onClick={() => onEnableChain(group)}>
									Enable chain
									<ChevronRightIcon aria-hidden="true" />
								</button>
							)}
						</>
					)}
				</div>
			</header>
			{group.vaults.map((known) => {
				const index = rowIndexOf(known.address)
				return (
					<CuratedVaultRow
						key={`${group.key}-${known.address}`}
						chain={group}
						known={known}
						row={index >= 0 ? rows[index] : undefined}
						enabled={group.enabled}
						onToggle={(selected) => onToggle(known, selected)}
						onChange={(changes) => onChange(index, changes)}
					/>
				)
			})}
			{customRows.map(({ row, index }) => (
				<CustomVaultRow
					key={`${row.chain}-${row.vault}-${index}`}
					row={row}
					onConfigure={() => onConfigureCustom(row, index)}
					onRemove={() => onRemoveCustom(index)}
				/>
			))}
		</section>
	)
}

/** Chains the filler does not run, folded behind one row until the operator wants them. */
function OtherNetworks(props: { groups: ChainGroup[]; children: ReactNode }) {
	const { groups, children } = props
	const [open, setOpen] = useState(false)
	const vaultCount = groups.reduce((sum, group) => sum + group.vaults.length, 0)
	return (
		<Collapsible.Root className="vault-others" open={open} onOpenChange={setOpen}>
			<Collapsible.Trigger asChild>
				<button type="button" className="vault-others-trigger">
					<span className="vault-group-identity">
						<span className="vault-others-logos" aria-hidden="true">
							{groups.map((group) => (
								<ChainLogo key={group.key} label={group.label} />
							))}
						</span>
						<span>
							<strong>Other networks</strong>
							<small>
								{groups.map((group) => group.label).join(", ")} · {vaultCount} curated{" "}
								{vaultCount === 1 ? "vault" : "vaults"}
							</small>
						</span>
					</span>
					<span className="vault-group-status">
						<span className="vault-status-pill">Not enabled</span>
						<span className="vault-others-chevron" aria-hidden="true">
							<ChevronDownIcon />
						</span>
					</span>
				</button>
			</Collapsible.Trigger>
			<Collapsible.Content className="vault-others-content">{children}</Collapsible.Content>
		</Collapsible.Root>
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
							label={vaultFieldLabel("threshold", "underlying token")}
							help={vaultFieldHelp("threshold", "underlying token")}
							value={draft.threshold}
							onChange={(threshold) => onChange({ ...draft, threshold })}
						/>
						<VaultAmountField
							label={vaultFieldLabel("minBalance", "underlying token")}
							help={vaultFieldHelp("minBalance", "underlying token")}
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
	/** Chains the filler runs (or the wizard enables): the only ones a vault can be saved for. */
	chains: VaultChainOption[]
	/** Catalog per chain; may cover chains beyond `chains`, which fold into "Other networks". */
	knownVaults: Record<string, KnownVault[]>
	/** Restricts the folded chains to this network; omit when the catalog is already network-scoped. */
	network?: InitNetwork
	/** Takes the operator to wherever the chain gets enabled; omit to hide the link. */
	onEnableChain?: (chain: VaultChainOption) => void
	rows: VaultRowDraft[]
	onChange: (rows: VaultRowDraft[]) => void
}) {
	const { chains, knownVaults, network, onEnableChain, rows, onChange } = props
	const [customDraft, setCustomDraft] = useState<CustomVaultDraft>()
	const groups = chainGroups(chains, knownVaults, network)
	const enabledGroups = groups.filter((group) => group.enabled)
	const otherGroups = groups.filter((group) => !group.enabled)
	const selectable = enabledGroups.flatMap((group) => group.vaults.map((known) => ({ group, known })))

	const rowIndexOf = (chainKey: string, address: string) =>
		rows.findIndex((row) => row.chain === chainKey && row.vault.toLowerCase() === address.toLowerCase())
	const knownFor = (row: VaultRowDraft): KnownVault | undefined =>
		(knownVaults[row.chain] ?? []).find((known) => known.address.toLowerCase() === row.vault.trim().toLowerCase())
	const patch = (index: number, changes: Partial<VaultRowDraft>) =>
		onChange(rows.map((row, rowIndex) => (rowIndex === index ? { ...row, ...changes } : row)))

	const allSelected =
		selectable.length > 0 && selectable.every(({ group, known }) => rowIndexOf(group.key, known.address) >= 0)
	const customRows = rows.reduce<Array<{ row: VaultRowDraft; index: number }>>((list, row, index) => {
		if (!knownFor(row)) list.push({ row, index })
		return list
	}, [])
	// Custom vaults sit inside their chain's group; a chain no longer enabled keeps
	// its own group so the row stays visible and removable.
	const customByChain = new Map<string, Array<{ row: VaultRowDraft; index: number }>>()
	for (const entry of customRows) {
		const list = customByChain.get(entry.row.chain) ?? []
		list.push(entry)
		customByChain.set(entry.row.chain, list)
	}
	const orphanChains = [...customByChain.keys()].filter((chainKey) => !groups.some((group) => group.key === chainKey))
	const unselectedCatalogRows = selectable.reduce<VaultRowDraft[]>((list, { group, known }) => {
		if (rowIndexOf(group.key, known.address) < 0) {
			list.push({ chain: group.key, vault: known.address, ...defaultsForKnownVault(known) })
		}
		return list
	}, [])

	const toggleAll = () =>
		onChange(
			allSelected
				? selectable.reduce((list, { group, known }) => withoutVaultRow(list, group.key, known.address), rows)
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

	const renderGroup = (group: ChainGroup) => (
		<VaultChainGroup
			key={group.key}
			group={group}
			rows={rows}
			customRows={customByChain.get(group.key) ?? []}
			onEnableChain={onEnableChain}
			onToggle={(known, selected) =>
				onChange(
					selected
						? [...rows, { chain: group.key, vault: known.address, ...defaultsForKnownVault(known) }]
						: withoutVaultRow(rows, group.key, known.address),
				)
			}
			onChange={patch}
			onConfigureCustom={(row, index) => setCustomDraft({ ...row, index })}
			onRemoveCustom={(index) => onChange(rows.filter((_, rowIndex) => rowIndex !== index))}
		/>
	)

	return (
		<div className="vault-editor">
			<div className="vault-editor-heading">
				<div>
					<h3>Curated vaults</h3>
					<p>Select a vault on an enabled chain, then set how much stays in the wallet.</p>
				</div>
				<div className="vault-editor-actions">
					{selectable.length > 1 && (
						<button type="button" className="market-text-action" onClick={toggleAll}>
							{allSelected ? "Clear curated vaults" : "Select all"}
						</button>
					)}
					<button type="button" className="market-create-button" onClick={openNewCustomVault}>
						<span aria-hidden="true">+</span> Custom vault
					</button>
				</div>
			</div>

			<div className="vault-groups">
				{enabledGroups.map(renderGroup)}
				{orphanChains.map((chainKey) =>
					renderGroup({ key: chainKey, label: chainKey, enabled: false, vaults: [] }),
				)}
				{otherGroups.length > 0 && <OtherNetworks groups={otherGroups}>{otherGroups.map(renderGroup)}</OtherNetworks>}

				{groups.length === 0 && customRows.length === 0 && (
					<div className="vault-empty-state">
						<strong>No curated vaults are available on this network.</strong>
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
