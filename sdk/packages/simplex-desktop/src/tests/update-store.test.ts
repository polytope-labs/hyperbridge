import { mkdtempSync, readFileSync, rmSync, statSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it } from "vitest"
import { FileUpdateStore } from "../update-store"

const directories: string[] = []

afterEach(() => {
	for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
})

describe("desktop update store", () => {
	it("writes channel and receipt atomically with private Unix permissions", () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-updates-"))
		directories.push(directory)
		const store = new FileUpdateStore(directory)
		store.write({
			channel: "beta",
			receipt: { fromVersion: "1.0.0", targetVersion: "1.1.0", downloadedAt: 42 },
		})
		expect(store.read()).toEqual({
			channel: "beta",
			receipt: { fromVersion: "1.0.0", targetVersion: "1.1.0", downloadedAt: 42 },
		})
		const path = join(directory, "desktop-updates.json")
		expect(JSON.parse(readFileSync(path, "utf8"))).toMatchObject({ channel: "beta" })
		if (process.platform !== "win32") expect(statSync(path).mode & 0o777).toBe(0o600)
	})

	it("falls back safely when the settings file is absent or malformed", () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-updates-"))
		directories.push(directory)
		const store = new FileUpdateStore(directory)
		expect(store.read()).toEqual({ channel: "stable" })
	})
})
