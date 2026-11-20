import {
	chmodSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readdirSync,
	rmSync,
	symlinkSync,
	writeFileSync,
} from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
	assertProfileCanBeReset,
	claimProfile,
	completePendingProfileReset,
	requestProfileReset,
} from "../profile-reset"

/** Stands in for the home directory, so no test touches the real one. */
const home = vi.hoisted(() => ({ path: "" }))
vi.mock("node:os", async (original) => ({
	...(await original<typeof import("node:os")>()),
	homedir: () => home.path,
}))

/** A file permission cannot stop a delete on Windows, or for root anywhere. */
const permissionsBind = process.platform !== "win32" && process.getuid?.() !== 0

let root: string
let profile: string

beforeEach(() => {
	root = mkdtempSync(join(tmpdir(), "simplex-reset-"))
	home.path = join(root, "home")
	profile = join(root, "profile")
	mkdirSync(home.path)
	mkdirSync(profile)
	// As at a first launch: the directory is empty, so Simplex stamps it as its own.
	claimProfile(profile, false)
	writeFileSync(join(profile, "filler-config.toml"), "encrypted")
	writeFileSync(join(profile, "desktop-vault.json"), "{}")
	writeFileSync(join(profile, "bids.db"), "rows")
	mkdirSync(join(profile, "logs"))
	writeFileSync(join(profile, "logs", "launch.ndjson"), "{}")
	mkdirSync(join(profile, "tunnel", "devices"), { recursive: true })
	writeFileSync(join(profile, "tunnel", "devices", "phone.pub"), "key")
})

afterEach(() => rmSync(root, { recursive: true, force: true }))

describe("profile reset", () => {
	it("leaves the profile alone until a reset is requested", () => {
		expect(completePendingProfileReset(profile)).toBe(false)
		expect(readdirSync(profile).sort()).toEqual([
			"bids.db",
			"desktop-vault.json",
			"filler-config.toml",
			"logs",
			"simplex-profile",
			"tunnel",
		])
	})

	it("deletes every file and directory once a reset is requested, and only once", () => {
		requestProfileReset(profile)
		expect(existsSync(join(profile, "filler-config.toml"))).toBe(true)

		// Only the stamp is left: the directory is still Simplex's, and can be reset again.
		expect(completePendingProfileReset(profile)).toBe(true)
		expect(readdirSync(profile)).toEqual(["simplex-profile"])

		writeFileSync(join(profile, "filler-config.toml"), "a new profile")
		expect(completePendingProfileReset(profile)).toBe(false)
		expect(readdirSync(profile).sort()).toEqual(["filler-config.toml", "simplex-profile"])

		requestProfileReset(profile)
		expect(completePendingProfileReset(profile)).toBe(true)
		expect(readdirSync(profile)).toEqual(["simplex-profile"])
	})

	it("requests a reset for a profile directory that does not exist yet", () => {
		const missing = join(profile, "never-created")
		requestProfileReset(missing)
		expect(completePendingProfileReset(missing)).toBe(true)
		expect(readdirSync(missing)).toEqual(["simplex-profile"])
	})

	it("refuses a directory that held files before Simplex was pointed at it", () => {
		const documents = join(root, "documents")
		mkdirSync(documents)
		writeFileSync(join(documents, "thesis.txt"), "not Simplex's")
		// A launch with `--user-data-dir` here adds Simplex's files, and no stamp.
		claimProfile(documents, false)
		writeFileSync(join(documents, "filler-config.toml"), "encrypted")

		expect(() => assertProfileCanBeReset(documents)).toThrow(/will not delete it/)
		expect(() => requestProfileReset(documents)).toThrow(/will not delete it/)
		expect(existsSync(join(documents, "reset-pending"))).toBe(false)

		// A marker that got in anyway deletes nothing.
		writeFileSync(join(documents, "reset-pending"), "")
		expect(completePendingProfileReset(documents)).toBe(false)
		expect(readdirSync(documents).sort()).toEqual(["filler-config.toml", "reset-pending", "thesis.txt"])
	})

	it("stamps the app's default directory although an earlier version already filled it", () => {
		const earlier = join(root, "earlier")
		mkdirSync(earlier)
		writeFileSync(join(earlier, "filler-config.toml"), "encrypted")

		claimProfile(earlier, true)
		requestProfileReset(earlier)
		expect(completePendingProfileReset(earlier)).toBe(true)
		expect(readdirSync(earlier)).toEqual(["simplex-profile"])
	})

	it("never stamps the home directory, a directory above it, or a link", () => {
		claimProfile(home.path, true)
		claimProfile(root, true)
		const linked = join(root, "linked-empty")
		mkdirSync(join(root, "empty"))
		symlinkSync(join(root, "empty"), linked)
		claimProfile(linked, true)

		expect(existsSync(join(home.path, "simplex-profile"))).toBe(false)
		expect(existsSync(join(root, "simplex-profile"))).toBe(false)
		expect(existsSync(join(root, "empty", "simplex-profile"))).toBe(false)
	})

	it.runIf(permissionsBind)("deletes what it can and stays pending when an entry cannot be deleted", () => {
		const held = join(profile, "held")
		mkdirSync(held)
		writeFileSync(join(held, "Cookies"), "open in another process")
		chmodSync(held, 0o500)
		requestProfileReset(profile)
		try {
			expect(() => completePendingProfileReset(profile, 0)).toThrow()
			expect(readdirSync(profile).sort()).toEqual(["held", "reset-pending", "simplex-profile"])
		} finally {
			chmodSync(held, 0o700)
		}

		expect(completePendingProfileReset(profile, 0)).toBe(true)
		expect(readdirSync(profile)).toEqual(["simplex-profile"])
	})

	it("honours a marker that is a directory, and removes it too", () => {
		mkdirSync(join(profile, "reset-pending"))
		expect(completePendingProfileReset(profile)).toBe(true)
		expect(readdirSync(profile)).toEqual(["simplex-profile"])
	})

	it.runIf(permissionsBind)("waits out the retry window before giving up on an entry", () => {
		const held = join(profile, "held")
		mkdirSync(held)
		writeFileSync(join(held, "Cookies"), "open in another process")
		chmodSync(held, 0o500)
		requestProfileReset(profile)
		const started = Date.now()
		try {
			expect(() => completePendingProfileReset(profile, 250)).toThrow()
		} finally {
			chmodSync(held, 0o700)
		}
		expect(Date.now() - started).toBeGreaterThanOrEqual(250)
	})

	it.runIf(process.platform !== "win32")("removes a link in the profile without following it", () => {
		const outside = join(root, "outside")
		mkdirSync(outside)
		writeFileSync(join(outside, "keep.txt"), "not Simplex's")
		symlinkSync(outside, join(profile, "linked"))
		requestProfileReset(profile)

		expect(completePendingProfileReset(profile)).toBe(true)
		expect(readdirSync(profile)).toEqual(["simplex-profile"])
		expect(readdirSync(outside)).toEqual(["keep.txt"])
	})

	it("refuses a directory that holds the home directory, and ignores a marker left in one", () => {
		expect(() => requestProfileReset(root)).toThrow(/will not delete it/)
		expect(() => requestProfileReset(home.path)).toThrow(/will not delete it/)
		expect(existsSync(join(home.path, "reset-pending"))).toBe(false)

		writeFileSync(join(home.path, "reset-pending"), "")
		writeFileSync(join(home.path, "documents.txt"), "not Simplex's")
		expect(completePendingProfileReset(home.path)).toBe(false)
		expect(readdirSync(home.path).sort()).toEqual(["documents.txt", "reset-pending"])
	})

	it("refuses the home directory even when it carries the stamp", () => {
		writeFileSync(join(home.path, "simplex-profile"), "")
		writeFileSync(join(home.path, "documents.txt"), "not Simplex's")
		expect(() => requestProfileReset(home.path)).toThrow(/will not delete it/)

		writeFileSync(join(home.path, "reset-pending"), "")
		expect(completePendingProfileReset(home.path)).toBe(false)
		expect(readdirSync(home.path).sort()).toEqual(["documents.txt", "reset-pending", "simplex-profile"])
	})

	it("says whether a reset would be refused without writing anything", () => {
		expect(() => assertProfileCanBeReset(profile)).not.toThrow()
		expect(() => assertProfileCanBeReset(home.path)).toThrow(/will not delete it/)
		expect(() => assertProfileCanBeReset(join(profile, "never-created"))).not.toThrow()
		expect(existsSync(join(profile, "reset-pending"))).toBe(false)
		expect(existsSync(join(home.path, "reset-pending"))).toBe(false)
	})

	it("refuses a profile that is a link, and one that reaches the home directory through a link", () => {
		writeFileSync(join(home.path, "documents.txt"), "not Simplex's")

		// The profile is a link to the home directory.
		const linked = join(root, "linked-profile")
		symlinkSync(home.path, linked)
		expect(() => requestProfileReset(linked)).toThrow(/will not delete it/)

		// A directory above the profile is a link to the directory above home, so the profile is home.
		const above = join(root, "above")
		symlinkSync(root, above)
		expect(() => requestProfileReset(join(above, "home"))).toThrow(/will not delete it/)

		// A link to a real profile is refused too: the reset would empty what it points at.
		const alias = join(root, "alias")
		symlinkSync(profile, alias)
		expect(() => requestProfileReset(alias)).toThrow(/will not delete it/)

		// A marker that got in anyway does not make the next launch delete through the link.
		writeFileSync(join(home.path, "reset-pending"), "")
		expect(completePendingProfileReset(linked)).toBe(false)
		expect(readdirSync(home.path).sort()).toEqual(["documents.txt", "reset-pending"])
		expect(existsSync(join(profile, "bids.db"))).toBe(true)
	})

	/** macOS and Windows open a path whatever its case, so `/users/bob` is `/Users/bob`. */
	it.runIf(process.platform === "darwin" || process.platform === "win32")(
		"refuses the home directory spelled in another case",
		() => {
			expect(() => requestProfileReset(home.path.toUpperCase())).toThrow(/will not delete it/)
		},
	)
})
