import type { RuntimeState, StateStore } from "./types"

/**
 * Merges `patch` into the stored runtime state. `StateStore.set` replaces the
 * whole record, so every writer must go through here or it wipes the other
 * writers' keys (a pause would otherwise forget the live phantom bids).
 */
export async function patchRuntimeState(store: StateStore, patch: Partial<RuntimeState>): Promise<RuntimeState> {
	const next = { ...(await store.get()), ...patch }
	await store.set(next)
	return next
}
