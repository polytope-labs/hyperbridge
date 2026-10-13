import { DEFAULT_ORDERBOOK_URLS } from "@/config/defaults"
import type { ChainConfirmationPolicy, FillerTomlConfig } from "@/config/filler-toml"
import { normalizeConfirmationPolicyKeys } from "./confirmation-keys"

/** The pre-limit-order strategy rows, as far as migration needs them. */
interface LegacyStrategy {
	type: "stable" | "hyperfx"
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
}

/**
 * Rewrites a config that predates limit orders in place. `[[strategies]]` is
 * dropped, because simplex now prices every fill from the limit orders the
 * operator posts. Per-strategy confirmation policies move top-level, and an
 * `[orderbook]` section is added when missing. Returns human-readable notes
 * about what moved.
 */
export function migrateLegacyConfig(config: FillerTomlConfig): string[] {
	const legacy = config as FillerTomlConfig & { strategies?: LegacyStrategy[] }
	const strategies = legacy.strategies
	if (!strategies?.length) {
		delete legacy.strategies
		return []
	}

	const notes: string[] = [
		"dropped [[strategies]] — simplex prices every fill from the limit orders you post from the dashboard",
	]
	// Keys are normalized on both sides so top-level entries genuinely win over
	// per-strategy ones — "EVM-8453" and "8453" are the same chain.
	const confirmationPolicies = normalizeConfirmationPolicyKeys(config.confirmationPolicies ?? {})
	for (const strategy of strategies) {
		for (const [chainId, policy] of Object.entries(
			normalizeConfirmationPolicyKeys(strategy.confirmationPolicies ?? {}),
		)) {
			confirmationPolicies[chainId] ??= policy
		}
	}

	delete legacy.strategies
	// A legacy config predates the orderbook, and simplex has no prices without one. The
	// wizard swaps this default for the selected network's before it writes the file.
	if (!config.orderbook) {
		config.orderbook = { url: DEFAULT_ORDERBOOK_URLS.mainnet }
		notes.push(
			`Added [orderbook] pointing at ${DEFAULT_ORDERBOOK_URLS.mainnet}; the wizard switches it to the selected network's orderbook.`,
		)
	}
	if (Object.keys(confirmationPolicies).length > 0) {
		config.confirmationPolicies = confirmationPolicies
	}
	return notes
}
