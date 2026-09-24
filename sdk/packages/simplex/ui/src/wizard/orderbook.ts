import type { Dispatch, SetStateAction } from "react"
import { api } from "../api"
import type { SetupOrderbook } from "../types"
import type { WizardState } from "./state"

/**
 * Reads the orderbook the config will post to, and the books it lists. Run when
 * the wizard opens, and again from the Chains step if that first read failed.
 */
export async function loadOrderbook(setState: Dispatch<SetStateAction<WizardState>>): Promise<void> {
	setState((s) => ({ ...s, orderbookError: undefined }))
	try {
		const orderbook = await api.get<SetupOrderbook>("/api/setup/orderbook")
		setState((s) => ({ ...s, orderbook }))
	} catch (err) {
		setState((s) => ({ ...s, orderbookError: err instanceof Error ? err.message : String(err) }))
	}
}
