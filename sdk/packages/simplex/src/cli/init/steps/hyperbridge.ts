import { HYPERBRIDGE_WS_DEFAULTS } from "../chains"
import { why, askSecret, askUrl } from "../prompt-utils"
import { WHY } from "../help-text"
import type { Prefill, WizardState } from "../state"

export async function stepHyperbridge(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.substrateKey)
	const key = await askSecret(
		"Substrate private key (hex seed or mnemonic phrase; leave empty to bid on the orderbook alone)",
		prefill?.config.simplex.substratePrivateKey || undefined,
		undefined,
		{ optional: true },
	)
	// Without a key there is nowhere to send Hyperbridge bids, so neither field is written.
	if (!key) {
		state.substratePrivateKey = undefined
		state.hyperbridgeWsUrl = undefined
		return
	}
	state.substratePrivateKey = key

	why(WHY.hyperbridgeWs)
	state.hyperbridgeWsUrl = await askUrl("Hyperbridge WebSocket URL", {
		initial: prefill?.config.simplex.hyperbridgeWsUrl || HYPERBRIDGE_WS_DEFAULTS[state.network],
		protocols: ["ws:", "wss:"],
	})
}
