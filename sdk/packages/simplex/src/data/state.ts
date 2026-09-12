import type { RuntimeState, StateStore } from "./types"

/**
 * Merges `patch` into the stored runtime state.
 *
 * Uses the store's own atomic merge when it has one. Otherwise `StateStore.set`
 * replaces the whole record, so every writer must come through here or it wipes
 * the other writers' keys (a pause would otherwise forget the live phantom
 * bids) — and even then two overlapping calls can lose one another's write.
 */
export async function patchRuntimeState(store: StateStore, patch: Partial<RuntimeState>): Promise<RuntimeState> {
	if (store.patch) return store.patch(patch)
	const next = { ...(await store.get()), ...patch }
	await store.set(next)
	return next
}
