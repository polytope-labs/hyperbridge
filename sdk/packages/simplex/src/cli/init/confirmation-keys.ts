import { log } from "@clack/prompts"
import type { ChainConfirmationPolicy } from "@/config/filler-toml"
import { parseChainKey } from "@/config/interpolated-curve"

/**
 * Normalizes a confirmation-policy map's keys to bare numeric strings so the
 * "EVM-8453" and "8453" spellings of one chain merge instead of colliding with
 * later-wins semantics. Unparseable keys are kept verbatim with a warning —
 * dropping them would silently disable a policy the operator wrote.
 */
export function normalizeConfirmationPolicyKeys(
	policies: Record<string, ChainConfirmationPolicy>,
): Record<string, ChainConfirmationPolicy> {
	const normalized: Record<string, ChainConfirmationPolicy> = {}
	for (const [key, policy] of Object.entries(policies)) {
		const parsed = parseChainKey(key)
		if (parsed === null) {
			log.warn(`Confirmation policy key '${key}' is not a chain id — kept as-is; \`simplex run\` will reject it.`)
			normalized[key] = policy
			continue
		}
		normalized[String(parsed)] = policy
	}
	return normalized
}
