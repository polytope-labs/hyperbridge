import { createRequire } from "node:module"
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { describe, expect, it, vi } from "vitest"
import { notarytoolArguments } from "./notarize-macos-dmg.mjs"
import { entitlementKeys, teamIdentifier } from "./verify-macos-signatures.mjs"
import { assertWindowsUpdatePublisher, verifyWindowsUpdateConfig } from "./verify-windows-update-config.mjs"

const require = createRequire(import.meta.url)
const {
	assertReleaseSigningEnvironment,
	loadBuilderConfig,
	releaseSigningEnabled,
	signPackagedWindowsRuntime,
	validateAzureEndpoint,
} = require("./release-signing.cjs")

async function macEnvironment() {
	const directory = await mkdtemp(join(tmpdir(), "simplex-signing-"))
	const apiKey = join(directory, "AuthKey_TEST.p8")
	await writeFile(apiKey, "fixture")
	return {
		SIMPLEX_DESKTOP_SIGN_RELEASE: "true",
		CSC_LINK: "base64-p12",
		CSC_KEY_PASSWORD: "password",
		APPLE_API_KEY: apiKey,
		APPLE_API_KEY_ID: "KEYID",
		APPLE_API_ISSUER: "issuer",
		APPLE_TEAM_ID: "TEAMID",
	}
}

function windowsEnvironment() {
	return {
		SIMPLEX_DESKTOP_SIGN_RELEASE: "true",
		AZURE_TENANT_ID: "tenant",
		AZURE_CLIENT_ID: "client",
		AZURE_CLIENT_SECRET: "secret",
		SIMPLEX_WINDOWS_PUBLISHER_NAME: "Polytope Labs",
		SIMPLEX_AZURE_SIGNING_ENDPOINT: "https://eus.codesigning.azure.net/",
		SIMPLEX_AZURE_SIGNING_ACCOUNT_NAME: "simplex",
		SIMPLEX_AZURE_CERTIFICATE_PROFILE_NAME: "desktop",
	}
}

describe("desktop release signing", () => {
	it("requires an explicit boolean signing mode", () => {
		expect(releaseSigningEnabled({})).toBe(false)
		expect(releaseSigningEnabled({ SIMPLEX_DESKTOP_SIGN_RELEASE: "true" })).toBe(true)
		expect(() => releaseSigningEnabled({ SIMPLEX_DESKTOP_SIGN_RELEASE: "yes" })).toThrow(/exactly/)
	})

	it("keeps ordinary and pull-request packages explicitly unsigned", () => {
		const mac = loadBuilderConfig({}, "darwin")
		expect(mac.mac).toMatchObject({ identity: null, notarize: false })
		const windows = loadBuilderConfig({}, "win32")
		expect(windows.win).toMatchObject({ signExecutable: false })
	})

	it("fails closed when signed macOS credentials are incomplete", async () => {
		const { APPLE_API_ISSUER: _missing, ...env } = await macEnvironment()
		expect(() => assertReleaseSigningEnvironment("darwin", env)).toThrow(/APPLE_API_ISSUER/)
	})

	it("configures the app and bundled Node with only the hardened-runtime entitlements", async () => {
		const config = loadBuilderConfig(await macEnvironment(), "darwin")
		expect(config.forceCodeSigning).toBe(true)
		expect(config.dmg).toMatchObject({ sign: true })
		expect(config.extraMetadata).toMatchObject({ simplexMacTeamId: "TEAMID" })
		expect(config.mac).toMatchObject({
			hardenedRuntime: true,
			entitlements: "resources/entitlements.mac.plist",
			entitlementsInherit: "resources/entitlements.mac.plist",
			preAutoEntitlements: false,
			strictVerify: true,
			binaries: ["Contents/Resources/runtime/node"],
			notarize: true,
		})
		const entitlements = entitlementKeys(
			await readFile(new URL("../resources/entitlements.mac.plist", import.meta.url), "utf8"),
		)
		expect(entitlements).toEqual([
			"com.apple.security.cs.allow-jit",
			"com.apple.security.cs.allow-unsigned-executable-memory",
		])
	})

	it("parses the exact macOS team and entitlement identity used by release verification", () => {
		expect(teamIdentifier("Authority=Developer ID Application\nTeamIdentifier=ABC123\n")).toBe("ABC123")
		expect(
			entitlementKeys(`
				<dict>
					<key>com.apple.security.cs.allow-unsigned-executable-memory</key><true/>
					<key>com.apple.security.cs.allow-jit</key><true/>
				</dict>
			`),
		).toEqual([
			"com.apple.security.cs.allow-jit",
			"com.apple.security.cs.allow-unsigned-executable-memory",
		])
	})

	it("submits the distributable DMG with the configured App Store Connect key", () => {
		expect(
			notarytoolArguments("/release/Simplex.dmg", {
				APPLE_API_KEY: "/tmp/AuthKey_TEST.p8",
				APPLE_API_KEY_ID: "KEYID",
				APPLE_API_ISSUER: "issuer",
			}),
		).toEqual([
			"notarytool",
			"submit",
			"/release/Simplex.dmg",
			"--key",
			"/tmp/AuthKey_TEST.p8",
			"--key-id",
			"KEYID",
			"--issuer",
			"issuer",
			"--wait",
			"--output-format",
			"json",
		])
	})

	it("accepts only Azure Trusted Signing service endpoints", () => {
		expect(validateAzureEndpoint("https://eus.codesigning.azure.net")).toBe(
			"https://eus.codesigning.azure.net/",
		)
		for (const endpoint of [
			"http://eus.codesigning.azure.net",
			"https://codesigning.azure.net.attacker.example",
			"https://user@eus.codesigning.azure.net",
			"https://eus.codesigning.azure.net/?profile=other",
		]) {
			expect(() => validateAzureEndpoint(endpoint)).toThrow()
		}
	})

	it("configures Azure Trusted Signing and updater publisher verification", () => {
		const config = loadBuilderConfig(windowsEnvironment(), "win32")
		expect(config.forceCodeSigning).toBe(true)
		expect(config.win).toMatchObject({
			signExecutable: true,
			signExts: [".exe"],
			verifyUpdateCodeSignature: true,
			azureSignOptions: {
				publisherName: "Polytope Labs",
				endpoint: "https://eus.codesigning.azure.net/",
				certificateProfileName: "desktop",
				codeSigningAccountName: "simplex",
				fileDigest: "SHA256",
				timestampDigest: "SHA256",
			},
		})
	})

	it("requires the packaged Windows updater configuration to retain the trusted publisher", async () => {
		expect(assertWindowsUpdatePublisher('publisherName: "Polytope Labs"\n', "Polytope Labs")).toEqual([
			"Polytope Labs",
		])
		expect(() => assertWindowsUpdatePublisher("provider: github\n", "Polytope Labs")).toThrow(/publisher/)
		expect(() => assertWindowsUpdatePublisher('publisherName: "Someone Else"\n', "Polytope Labs")).toThrow(
			/Polytope Labs/,
		)

		const root = await mkdtemp(join(tmpdir(), "simplex-windows-update-config-"))
		const resources = join(root, "win-unpacked", "resources")
		await mkdir(resources, { recursive: true })
		const config = join(resources, "app-update.yml")
		await writeFile(config, 'publisherName: "Polytope Labs"\n')
		await expect(verifyWindowsUpdateConfig(root, "Polytope Labs")).resolves.toBe(config)
	})

	it("signs the Windows Node runtime after staging and rejects a skipped signature", async () => {
		const signIf = vi.fn().mockResolvedValue(true)
		const context = { electronPlatformName: "win32", packager: { signIf } }
		await expect(signPackagedWindowsRuntime("C:\\Simplex\\runtime\\node.exe", context, windowsEnvironment())).resolves.toBe(
			true,
		)
		expect(signIf).toHaveBeenCalledWith("C:\\Simplex\\runtime\\node.exe")

		signIf.mockResolvedValue(false)
		await expect(
			signPackagedWindowsRuntime("C:\\Simplex\\runtime\\node.exe", context, windowsEnvironment()),
		).rejects.toThrow(/did not sign/)
	})

	it("does not invoke native signing for unsigned or Linux packages", async () => {
		const signIf = vi.fn()
		await expect(
			signPackagedWindowsRuntime("node.exe", { electronPlatformName: "win32", packager: { signIf } }, {}),
		).resolves.toBe(false)
		await expect(
			signPackagedWindowsRuntime(
				"node",
				{ electronPlatformName: "linux", packager: { signIf } },
				{ SIMPLEX_DESKTOP_SIGN_RELEASE: "true" },
			),
		).resolves.toBe(false)
		expect(signIf).not.toHaveBeenCalled()
	})
})
