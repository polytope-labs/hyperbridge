import { mkdtempSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it } from "vitest"
import { markSessionEnded, markSessionStarted, previousUncleanSession } from "../session-marker"

const directories: string[] = []

afterEach(() => {
	for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
})

function dataDir(): string {
	const directory = mkdtempSync(join(tmpdir(), "simplex-session-"))
	directories.push(directory)
	return directory
}

describe("desktop session marker", () => {
	const session = { version: "0.17.4", startedAt: "2026-10-14T12:00:00.000Z", platform: "win32" as const }
	const bootedBefore = Date.parse("2026-10-14T08:00:00.000Z")
	const bootedAfter = Date.parse("2026-10-14T13:00:00.000Z")

	it("finds no unclean session on a first launch or after a clean quit", () => {
		const directory = dataDir()
		expect(previousUncleanSession(directory, bootedBefore)).toBeUndefined()
		markSessionStarted(directory, session)
		markSessionEnded(directory)
		expect(previousUncleanSession(directory, bootedBefore)).toBeUndefined()
	})

	it("returns the session that never quit", () => {
		const directory = dataDir()
		markSessionStarted(directory, session)
		expect(previousUncleanSession(directory, bootedBefore)).toEqual(session)
	})

	it("ignores a session that an OS shutdown or restart ended", () => {
		const directory = dataDir()
		markSessionStarted(directory, session)
		expect(previousUncleanSession(directory, bootedAfter)).toBeUndefined()
	})

	it("still reports a marker it cannot parse", () => {
		const directory = dataDir()
		writeFileSync(join(directory, "running-session.json"), "{trunc")
		expect(previousUncleanSession(directory, bootedAfter)).toEqual({})
	})

	it("creates a missing data directory and ends a session that has no marker", () => {
		const directory = join(dataDir(), "fresh")
		markSessionStarted(directory, session)
		markSessionEnded(directory)
		expect(() => markSessionEnded(directory)).not.toThrow()
	})
})
