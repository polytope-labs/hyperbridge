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

/**
 * Marks a directory as one Simplex set up for itself. A reset deletes nothing from a directory
 * without it.
 *
 * `--user-data-dir` can point at a directory that already holds other files, such as a documents
 * folder or a drive. That directory is never stamped. A reset refuses it, and a file named
 * `reset-pending` left in it does nothing.
 */
const OWNERSHIP_STAMP = "simplex-profile"

/** How long the wipe keeps retrying files that the last session's helper processes still hold. */
const RETRY_WINDOW_MS = 3_000
const RETRY_PAUSE_MS = 100

/**
 * Whether a directory can be a profile at all, going by where it is.
 *
 * `--user-data-dir` can point anywhere. The home directory, or a directory above it, holds
 * other things, and the reset deletes everything it finds.
 *
 * The two are compared by their true paths. A path as it was given follows no links and keeps its
 * spelling, and macOS and Windows open `/users/bob` as `/Users/bob`. A profile that is itself a
 * link is refused, since the reset would empty whatever it points at.
 */
function canBeProfile(userDataDir: string): boolean {
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

/**
 * Stamps the profile as Simplex's own where it can be nothing else: it is the directory the
 * system gives this app, or it holds nothing yet. Called at every launch, before anything is
 * written to the profile.
 *
 * A profile under `--user-data-dir` that an earlier version filled has no stamp and gets none, so
 * it cannot be reset from the menu.
 */
export function claimProfile(userDataDir: string, isDefault: boolean): void {
	mkdirSync(userDataDir, { recursive: true })
	if (!canBeProfile(userDataDir)) return
	const stamp = join(userDataDir, OWNERSHIP_STAMP)
	if (existsSync(stamp)) return
	if (isDefault || readdirSync(userDataDir).length === 0) writeFileSync(stamp, "", { mode: 0o600 })
}

/** Whether everything in a directory is Simplex's to delete: it can be a profile, and Simplex stamped it. */
function holdsOnlySimplex(userDataDir: string): boolean {
	try {
		return canBeProfile(userDataDir) && lstatSync(join(userDataDir, OWNERSHIP_STAMP)).isFile()
	} catch {
		return false
	}
}

/** Throws when a reset would refuse this directory, so a caller can find out before it stops anything. */
export function assertProfileCanBeReset(userDataDir: string): void {
	// A directory that is not there holds nothing. A reset creates it before it writes its marker.
	if (!existsSync(userDataDir)) return
	if (!holdsOnlySimplex(userDataDir)) {
		throw new Error(
			`Simplex did not set up ${resolve(userDataDir)} as its own directory, so it will not delete it. To reset, choose Stop solver and quit, then delete Simplex's files there yourself`,
		)
	}
}

/** Records that the next launch must delete the profile. */
export function requestProfileReset(userDataDir: string): void {
	// A directory that was missing is created here, empty, and so is Simplex's own.
	claimProfile(userDataDir, false)
	assertProfileCanBeReset(userDataDir)
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
 * again on the next launch and never leaves half a profile to start from. The stamp is kept: the
 * directory is still Simplex's.
 */
export function completePendingProfileReset(userDataDir: string, retryWindowMs = RETRY_WINDOW_MS): boolean {
	const marker = join(userDataDir, RESET_MARKER)
	if (!existsSync(marker) || !holdsOnlySimplex(userDataDir)) return false
	const deadline = Date.now() + retryWindowMs
	let failure: unknown
	for (const entry of readdirSync(userDataDir)) {
		if (entry === RESET_MARKER || entry === OWNERSHIP_STAMP) continue
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
