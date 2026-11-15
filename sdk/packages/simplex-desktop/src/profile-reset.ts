import { existsSync, lstatSync, mkdirSync, readdirSync, realpathSync, rmSync, writeFileSync } from "node:fs"
import { homedir } from "node:os"
import { isAbsolute, join, relative, resolve } from "node:path"

/**
 * Exists only between a confirmed reset and the launch that carries it out.
 *
 * The profile is not deleted by the session that confirms the reset: while the app runs,
 * Chromium keeps files in this directory open, and Windows refuses to delete an open file.
 * The next launch deletes it before anything opens it.
 */
const RESET_MARKER = "reset-pending"

/** How long the wipe keeps retrying files that the last session's helper processes still hold. */
const RETRY_WINDOW_MS = 3_000
const RETRY_PAUSE_MS = 100

/**
 * Whether a directory can be one that holds only Simplex's files.
 *
 * `--user-data-dir` can point anywhere. The home directory, or a directory above it, holds
 * other things, and the reset deletes everything it finds.
 *
 * The two are compared by their true paths. A path as it was given follows no links and keeps its
 * spelling, and macOS and Windows open `/users/bob` as `/Users/bob`. A profile that is itself a
 * link is refused, since the reset would empty whatever it points at.
 */
function holdsOnlySimplex(userDataDir: string): boolean {
	try {
		if (lstatSync(userDataDir).isSymbolicLink()) return false
		const toHome = relative(realpathSync.native(userDataDir), realpathSync.native(homedir()))
		const homeIsInside = toHome === "" || (!toHome.startsWith("..") && !isAbsolute(toHome))
		return !homeIsInside
	} catch {
		// A directory that cannot be read cannot be shown to hold only Simplex's files.
		return false
	}
}

/** Records that the next launch must delete the profile. */
export function requestProfileReset(userDataDir: string): void {
	// Created first: the check reads the directory's true path, which a missing one does not have.
	mkdirSync(userDataDir, { recursive: true })
	if (!holdsOnlySimplex(userDataDir)) {
		throw new Error(`${resolve(userDataDir)} holds more than Simplex's files, so Simplex will not delete it`)
	}
	writeFileSync(join(userDataDir, RESET_MARKER), "", { mode: 0o600 })
}

/** Blocks the thread: the wipe has to finish before anything opens the profile. */
function pause(milliseconds: number): void {
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds)
}

/** Deletes one entry, retrying until the deadline. `rmSync`'s own retries do not wait on Windows. */
function removeBefore(path: string, deadline: number): void {
	for (;;) {
		try {
			rmSync(path, { recursive: true, force: true })
			return
		} catch (error) {
			if (Date.now() >= deadline) throw error
			pause(RETRY_PAUSE_MS)
		}
	}
}

/**
 * Deletes everything in the profile when a reset is pending, and says whether one was.
 *
 * Every entry is tried, so one that cannot be deleted does not keep the rest on disk. The marker
 * is deleted last and only when nothing else is left, so a reset that fails part-way is tried
 * again on the next launch and never leaves half a profile to start from.
 */
export function completePendingProfileReset(userDataDir: string, retryWindowMs = RETRY_WINDOW_MS): boolean {
	const marker = join(userDataDir, RESET_MARKER)
	if (!existsSync(marker) || !holdsOnlySimplex(userDataDir)) return false
	const deadline = Date.now() + retryWindowMs
	let failure: unknown
	for (const entry of readdirSync(userDataDir)) {
		if (entry === RESET_MARKER) continue
		try {
			removeBefore(join(userDataDir, entry), deadline)
		} catch (error) {
			failure ??= error
		}
	}
	if (failure) throw failure
	rmSync(marker, { recursive: true, force: true })
	return true
}
