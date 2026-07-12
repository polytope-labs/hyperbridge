import { text } from "@clack/prompts"
import { HYPERBRIDGE_WS_DEFAULTS } from "../chains"
import { guard, why, isValidUrl } from "../prompt-utils"
import { WHY } from "../help-text"
import { askSecret } from "./signer"
import type { Prefill, WizardState } from "../state"

export async function stepHyperbridge(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.substrateKey)
	state.substratePrivateKey = await askSecret(
		"Substrate private key (hex seed or mnemonic phrase)",
		prefill?.config.simplex.substratePrivateKey || undefined,
	)

	why(WHY.hyperbridgeWs)
	const wsUrl = guard(
		await text({
			message: "Hyperbridge WebSocket URL",
			initialValue: prefill?.config.simplex.hyperbridgeWsUrl || HYPERBRIDGE_WS_DEFAULTS[state.network],
			validate: (value) =>
				isValidUrl((value ?? "").trim(), ["ws:", "wss:"]) ? undefined : "Enter a valid ws(s):// URL",
		}),
	)
	state.hyperbridgeWsUrl = wsUrl.trim()
}
