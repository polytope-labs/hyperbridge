import { HYPERBRIDGE_WS_DEFAULTS } from "../chains"
import { why, askSecret, askUrl } from "../prompt-utils"
import { WHY } from "../help-text"
import type { Prefill, WizardState } from "../state"

export async function stepHyperbridge(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.substrateKey)
	state.substratePrivateKey = await askSecret(
		"Substrate private key (hex seed or mnemonic phrase)",
		prefill?.config.simplex.substratePrivateKey || undefined,
	)

	why(WHY.hyperbridgeWs)
	state.hyperbridgeWsUrl = await askUrl("Hyperbridge WebSocket URL", {
		initial: prefill?.config.simplex.hyperbridgeWsUrl || HYPERBRIDGE_WS_DEFAULTS[state.network],
		protocols: ["ws:", "wss:"],
	})
}
