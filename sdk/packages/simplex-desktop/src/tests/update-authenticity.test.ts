import { mkdirSync, writeFileSync } from "node:fs"
import { mkdtemp } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { describe, expect, it, vi } from "vitest"
import { updateAuthenticityForInstallation } from "../update-authenticity"

async function resources(config?: string): Promise<string> {
	const directory = await mkdtemp(join(tmpdir(), "simplex-update-authenticity-"))
	mkdirSync(directory, { recursive: true })
	if (config !== undefined) writeFileSync(join(directory, "app-update.yml"), config)
	return directory
}

describe("desktop update authenticity", () => {
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
			verifyMacSignature: verify,
		})
		expect(verify).toHaveBeenCalledWith("/Applications/Simplex.app")
		expect(result.enabled).toBe(false)
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
