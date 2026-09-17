import { mkdirSync, writeFileSync } from "node:fs"
import { mkdtemp } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { describe, expect, it, vi } from "vitest"
import {
	macApplicationBundlePath,
	macCodeSigningRequirement,
	macTeamIdFromAppPackage,
	updateAuthenticityForInstallation,
} from "../update-authenticity"

async function resources(config?: string): Promise<string> {
	const directory = await mkdtemp(join(tmpdir(), "simplex-update-authenticity-"))
	mkdirSync(directory, { recursive: true })
	if (config !== undefined) writeFileSync(join(directory, "app-update.yml"), config)
	return directory
}

describe("desktop update authenticity", () => {
	it("resolves macOS bundles independently of the test host", () => {
		expect(macApplicationBundlePath("/Applications/Simplex.app/Contents/MacOS/Simplex")).toBe(
			"/Applications/Simplex.app",
		)
	})

	it("disables Windows updates when publisher verification is absent", async () => {
		const result = updateAuthenticityForInstallation({
			packaged: true,
			platform: "win32",
			resourcesPath: await resources("provider: github\n"),
			executablePath: "C:\\Program Files\\Simplex\\Simplex.exe",
		})
		expect(result).toEqual({ enabled: false, reason: "The installed build has no trusted Windows publisher" })
	})

	it("enables Windows updates only with a non-empty publisher identity", async () => {
		const result = updateAuthenticityForInstallation({
			packaged: true,
			platform: "win32",
			resourcesPath: await resources('publisherName: "Polytope Labs"\n'),
			executablePath: "C:\\Program Files\\Simplex\\Simplex.exe",
		})
		expect(result).toEqual({ enabled: true })
	})

	it("requires a valid macOS application signature", async () => {
		const verify = vi.fn(() => false)
		const result = updateAuthenticityForInstallation({
			packaged: true,
			platform: "darwin",
			resourcesPath: "/Applications/Simplex.app/Contents/Resources",
			executablePath: "/Applications/Simplex.app/Contents/MacOS/Simplex",
			expectedMacTeamId: "ABCDE12345",
			verifyMacSignature: verify,
		})
		expect(verify).toHaveBeenCalledWith(
			"/Applications/Simplex.app",
			"=anchor apple generic and certificate leaf[subject.OU] = ABCDE12345",
		)
		expect(result.enabled).toBe(false)
	})

	it("requires the expected macOS Team ID before enabling updates", () => {
		const verify = vi.fn(() => true)
		expect(
			updateAuthenticityForInstallation({
				packaged: true,
				platform: "darwin",
				resourcesPath: "/Applications/Simplex.app/Contents/Resources",
				executablePath: "/Applications/Simplex.app/Contents/MacOS/Simplex",
				verifyMacSignature: verify,
			}),
		).toEqual({ enabled: false, reason: "The installed macOS application has no trusted Apple Team ID" })
		expect(verify).not.toHaveBeenCalled()
	})

	it("reads only a valid Team ID from signed app metadata", async () => {
		const appPath = await mkdtemp(join(tmpdir(), "simplex-app-metadata-"))
		writeFileSync(join(appPath, "package.json"), JSON.stringify({ simplexMacTeamId: "ABCDE12345" }))
		expect(macTeamIdFromAppPackage(appPath)).toBe("ABCDE12345")
		writeFileSync(join(appPath, "package.json"), JSON.stringify({ simplexMacTeamId: "not a team id" }))
		expect(macTeamIdFromAppPackage(appPath)).toBeUndefined()
		expect(macCodeSigningRequirement("ABCDE12345")).toBe(
			"=anchor apple generic and certificate leaf[subject.OU] = ABCDE12345",
		)
	})

	it("keeps Linux automatic installation disabled without signed metadata", () => {
		expect(
			updateAuthenticityForInstallation({
				packaged: true,
				platform: "linux",
				resourcesPath: "/opt/Simplex/resources",
				executablePath: "/opt/Simplex/simplex",
			}),
		).toEqual({
			enabled: false,
			reason: "Linux automatic updates require independently signed release metadata",
		})
	})
})
