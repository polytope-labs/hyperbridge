import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs"
import { join } from "node:path"

/**
 * A file that exists only while the desktop runs. Finding one at launch means
 * the previous session ended without a clean quit: a native crash or a forced
 * kill. Native crashes leave no other trace because Simplex collects no
 * minidumps. A session that started before the OS last booted was ended by a
 * shutdown or restart (Windows does not always emit will-quit then), not a crash.
 */
export interface DesktopSession {
	version: string
	startedAt: string
	platform: NodeJS.Platform
}

const MARKER_FILE = "running-session.json"

function markerPath(dataDir: string): string {
	return join(dataDir, MARKER_FILE)
}

/** Returns the session that did not quit cleanly, if any. A corrupt marker still counts. */
export function previousUncleanSession(dataDir: string, bootedAtMs: number): Partial<DesktopSession> | undefined {
	let raw: string
	try {
		raw = readFileSync(markerPath(dataDir), "utf8")
	} catch {
		return undefined
	}
	try {
		const parsed = JSON.parse(raw) as Partial<DesktopSession>
		if (!parsed || typeof parsed !== "object") return {}
		const startedAt = Date.parse(parsed.startedAt ?? "")
		return Number.isFinite(startedAt) && startedAt < bootedAtMs ? undefined : parsed
	} catch {
		return {}
	}
}

export function markSessionStarted(dataDir: string, session: DesktopSession): void {
	mkdirSync(dataDir, { recursive: true })
	writeFileSync(markerPath(dataDir), JSON.stringify(session), { mode: 0o600 })
}

export function markSessionEnded(dataDir: string): void {
	rmSync(markerPath(dataDir), { force: true })
}
