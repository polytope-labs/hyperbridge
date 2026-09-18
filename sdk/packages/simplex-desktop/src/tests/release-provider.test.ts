import type { AppUpdater } from "electron-updater"
import type { ProviderRuntimeOptions } from "electron-updater/out/providers/Provider.js"
import { describe, expect, it, vi } from "vitest"
import { resolveTrustedReleaseFiles, selectSimplexRelease, SimplexReleaseProvider } from "../release-provider"

describe("Simplex desktop release provider", () => {
	const releases = [
		{ tag_name: "tesseract-v9.0.0", draft: false, prerelease: false },
		{ tag_name: "simplex-v8.0.0", draft: false, prerelease: false },
		{ tag_name: "simplex-desktop-v1.4.0", draft: false, prerelease: false },
		{ tag_name: "simplex-desktop-v1.10.0", draft: false, prerelease: false },
		{ tag_name: "simplex-desktop-v1.5.0", draft: true, prerelease: false },
		{ tag_name: "simplex-desktop-v1.6.0-beta.1", draft: false, prerelease: true },
		{ tag_name: "simplex-desktop-v1.6.0-beta.3", draft: false, prerelease: true },
		{ tag_name: "simplex-desktop-v1.6.0-alpha.9", draft: false, prerelease: true },
	]

	it("selects the newest stable desktop tag without considering other monorepo releases", () => {
		expect(selectSimplexRelease(releases, "latest")?.tag_name).toBe("simplex-desktop-v1.10.0")
	})

	it("selects the newest beta desktop tag and excludes other prerelease channels", () => {
		expect(selectSimplexRelease(releases, "beta")?.tag_name).toBe("simplex-desktop-v1.6.0-beta.3")
	})

	it("loads updater metadata and resolves artifacts from the selected prefixed release", async () => {
		const request = vi.fn(async (options: { path?: string }) => {
			if (options.path?.startsWith("/repos/")) {
				return JSON.stringify([
					{ tag_name: "tesseract-v9.0.0", draft: false, prerelease: false },
					{ tag_name: "simplex-desktop-v1.10.0", draft: false, prerelease: false },
				])
			}
			return "version: 1.10.0\nfiles:\n  - url: Simplex-1.10.0.dmg\n    sha512: test-hash\n"
		})
		const provider = new SimplexReleaseProvider({ provider: "custom" }, { channel: "latest" } as AppUpdater, {
			platform: "darwin",
			isUseMultipleRangeRequest: false,
			executor: { request } as unknown as ProviderRuntimeOptions["executor"],
		})

		const info = await provider.getLatestVersion()
		expect(info).toMatchObject({ version: "1.10.0", tag: "simplex-desktop-v1.10.0" })
		expect(provider.resolveFiles(info)[0].url.href).toBe(
			"https://github.com/polytope-labs/hyperbridge/releases/download/simplex-desktop-v1.10.0/Simplex-1.10.0.dmg",
		)
	})

	it.each([
		"http://attacker.invalid/Simplex-1.10.0.dmg",
		"https://attacker.invalid/Simplex-1.10.0.dmg",
		"//attacker.invalid/Simplex-1.10.0.dmg",
		"../Simplex-1.10.0.dmg",
		"/polytope-labs/hyperbridge/releases/download/other-tag/Simplex-1.10.0.dmg",
		"Simplex-1.10.0.dmg?download=attacker",
	])("rejects an updater artifact outside the selected GitHub release: %s", (url) => {
		expect(() =>
			resolveTrustedReleaseFiles(
				{
					version: "1.10.0",
					tag: "simplex-desktop-v1.10.0",
					path: url,
					sha512: "hash",
					releaseDate: "2026-09-17T00:00:00.000Z",
					files: [{ url, sha512: "hash" }],
				},
				new URL("https://github.com/polytope-labs/hyperbridge/releases/download/simplex-desktop-v1.10.0/"),
			),
		).toThrow(/untrusted artifact path|escaped its pinned GitHub release/)
	})
})
