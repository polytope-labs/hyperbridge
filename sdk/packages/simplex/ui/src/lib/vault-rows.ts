import type { FillerConfig } from "../types"

/** One editable ERC-4626 vault row, shared by the wizard Treasury step and the operator Operations card. */
export interface VaultRowDraft {
	chain: string
	vault: string
	threshold: string
	minBalance: string
	redeemOnShutdown: boolean
}

export type VaultTomlRow = NonNullable<NonNullable<FillerConfig["vault"]>["vaults"]>[number]

/** Draft rows to TOML entries; blank optional fields are omitted, rows without an address dropped. */
export function vaultRowsToToml(rows: VaultRowDraft[]): VaultTomlRow[] {
	return rows
		.filter((row) => row.vault.trim())
		.map((row) => ({
			chain: row.chain,
			vault: row.vault.trim() as VaultTomlRow["vault"],
			...(row.threshold.trim() ? { threshold: row.threshold.trim() } : {}),
			...(row.minBalance.trim() ? { minBalance: row.minBalance.trim() } : {}),
			...(row.redeemOnShutdown ? { redeemOnShutdown: true } : {}),
		}))
}
