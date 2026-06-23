// ERC-4626 vault addresses per chain, keyed by underlying token address (lowercase).
// Add vault addresses here as new yield integrations are deployed.
// Values are arrays because multiple vaults may wrap the same underlying token.
export const YIELD_VAULT_ADDRESSES: Record<string, Record<string, string[]>> = {
	"EVM-1": {},
	"EVM-10": {},
	"EVM-56": {},
	"EVM-100": {},
	"EVM-130": {},
	"EVM-137": {},
	"EVM-8453": {},
	"EVM-42161": {},
}
