/**
 * Validates raw TOML `[vault].vaults` entries. Pure and browser-safe — shared
 * by `validateConfig`, the vault-update endpoint and the funding planner.
 * Throws on missing/invalid required fields.
 */
export function validateVaultToml(
	vaults: { chain?: string; vault?: string; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }[],
): void {
	const positiveNumber = (v: string) => /^\d+(\.\d+)?$/.test(v.trim()) && Number(v) > 0
	for (const v of vaults) {
		if (!v.chain?.trim()) {
			throw new Error("Each vault must have a non-empty 'chain' (e.g. EVM-8453)")
		}
		if (!v.vault?.trim()) {
			throw new Error("Each vault entry must include a 'vault' address")
		}
		if (v.threshold !== undefined && !positiveNumber(v.threshold)) {
			throw new Error(`Vault ${v.vault} 'threshold' must be a positive number`)
		}
		if (v.minBalance !== undefined && !positiveNumber(v.minBalance)) {
			throw new Error(`Vault ${v.vault} 'minBalance' must be a positive number`)
		}
		// Sweeping needs a floor to keep gas/paymaster funds, and a trigger
		// strictly above it so a sweep never tries to deposit ≤ 0.
		if (v.threshold !== undefined) {
			if (v.minBalance === undefined) {
				throw new Error(`Vault ${v.vault} sets 'threshold' so it must also set 'minBalance'`)
			}
			if (Number(v.threshold) <= Number(v.minBalance)) {
				throw new Error(`Vault ${v.vault} 'threshold' must be greater than 'minBalance'`)
			}
		}
	}
}
