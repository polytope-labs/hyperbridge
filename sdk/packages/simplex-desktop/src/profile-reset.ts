import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs"
import { join } from "node:path"

/**
 * Exists only between a confirmed reset and the launch that carries it out.
 *
 * The profile is not deleted by the session that confirms the reset: while the app runs,
 * Chromium keeps files in this directory open, and Windows refuses to delete an open file.
 * The next launch deletes it before anything opens it.
 */
const RESET_MARKER = "reset-pending"

/** Records that the next launch must delete the profile. */
export function requestProfileReset(userDataDir: string): void {
	mkdirSync(userDataDir, { recursive: true })
	writeFileSync(join(userDataDir, RESET_MARKER), "", { mode: 0o600 })
}

/**
 * Deletes everything in the profile when a reset is pending, and says whether one was.
 *
 * The marker is deleted last, so a reset that fails part-way is tried again on the next launch
 * and never leaves half a profile to start from.
 */
export function completePendingProfileReset(userDataDir: string): boolean {
	const marker = join(userDataDir, RESET_MARKER)
	if (!existsSync(marker)) return false
	for (const entry of readdirSync(userDataDir)) {
		if (entry === RESET_MARKER) continue
		// The last session's helper processes can hold a file for a moment after it quits.
		rmSync(join(userDataDir, entry), { recursive: true, force: true, maxRetries: 10, retryDelay: 200 })
	}
	rmSync(marker, { force: true })
	return true
}
