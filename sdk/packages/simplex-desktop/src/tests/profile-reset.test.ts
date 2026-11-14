import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import { completePendingProfileReset, requestProfileReset } from "../profile-reset"

let profile: string

beforeEach(() => {
	profile = mkdtempSync(join(tmpdir(), "simplex-reset-"))
	writeFileSync(join(profile, "filler-config.toml"), "encrypted")
	writeFileSync(join(profile, "desktop-vault.json"), "{}")
	writeFileSync(join(profile, "bids.db"), "rows")
	mkdirSync(join(profile, "logs"))
	writeFileSync(join(profile, "logs", "launch.ndjson"), "{}")
	mkdirSync(join(profile, "tunnel", "devices"), { recursive: true })
	writeFileSync(join(profile, "tunnel", "devices", "phone.pub"), "key")
})

afterEach(() => rmSync(profile, { recursive: true, force: true }))

describe("profile reset", () => {
	it("leaves the profile alone until a reset is requested", () => {
		expect(completePendingProfileReset(profile)).toBe(false)
		expect(readdirSync(profile).sort()).toEqual([
			"bids.db",
			"desktop-vault.json",
			"filler-config.toml",
			"logs",
			"tunnel",
		])
	})

	it("deletes every file and directory once a reset is requested, and only once", () => {
		requestProfileReset(profile)
		expect(existsSync(join(profile, "filler-config.toml"))).toBe(true)

		expect(completePendingProfileReset(profile)).toBe(true)
		expect(readdirSync(profile)).toEqual([])

		writeFileSync(join(profile, "filler-config.toml"), "a new profile")
		expect(completePendingProfileReset(profile)).toBe(false)
		expect(readdirSync(profile)).toEqual(["filler-config.toml"])
	})

	it("requests a reset for a profile directory that does not exist yet", () => {
		const missing = join(profile, "never-created")
		requestProfileReset(missing)
		expect(completePendingProfileReset(missing)).toBe(true)
		expect(readdirSync(missing)).toEqual([])
	})
})
