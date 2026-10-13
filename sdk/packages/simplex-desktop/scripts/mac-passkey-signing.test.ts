import { createRequire } from "node:module"
import { mkdtempSync, readFileSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { describe, expect, it } from "vitest"
const { validateMacPasskeyProfile, prepareMacPasskeySigning } = createRequire(import.meta.url)(
	"./mac-passkey-signing.cjs",
)
const policy = { rpId: "passkeys.example.com", teamId: "TEAM123" }
function profile() {
	return {
		teams: ["TEAM123"],
		platforms: ["OSX"],
		provisionsAllDevices: true,
		expiresAt: "2099-01-01T00:00:00Z",
		entitlements: {
			"com.apple.application-identifier": "PREFIX.network.hyperbridge.simplex",
			"com.apple.developer.team-identifier": "TEAM123",
			"com.apple.developer.associated-domains": ["*"],
		},
	}
}
describe("Mac passkey distribution identity", () => {
	it.each([
		["scalar wildcard", "*"],
		["array wildcard", ["*"]],
		["explicit domain", [`webcredentials:${policy.rpId}`]],
	])("accepts a profile authorized by %s", (_name, domains) => {
		const value = profile()
		expect(
			validateMacPasskeyProfile(
				{
					...value,
					entitlements: { ...value.entitlements, "com.apple.developer.associated-domains": domains },
				},
				policy,
			),
		).toBe("PREFIX.network.hyperbridge.simplex")
	})
	it.each([
		{ label: "missing", domains: undefined },
		{ label: "null", domains: null },
		{ label: "boolean", domains: true },
		{ label: "object", domains: {} },
		{ label: "empty array", domains: [] },
		{ label: "unrelated scalar", domains: "webcredentials:other.example" },
		{ label: "unrelated array", domains: ["webcredentials:other.example"] },
	])("rejects $label associated-domain permission", ({ domains }) => {
		const value = profile()
		expect(() =>
			validateMacPasskeyProfile(
				{
					...value,
					entitlements: { ...value.entitlements, "com.apple.developer.associated-domains": domains },
				},
				policy,
			),
		).toThrow("Apple profile does not authorize the passkey associated domain")
	})
	it("scopes generated app entitlements to the RP even when the profile authorizes a scalar wildcard", () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-passkey-signing-"))
		try {
			const value = profile()
			const result = prepareMacPasskeySigning(
				{
					APPLE_TEAM_ID: policy.teamId,
					SIMPLEX_PASSKEY_RP_ID: policy.rpId,
					SIMPLEX_MACOS_PROVISIONING_PROFILE_PATH: join(directory, "profile.provisionprofile"),
				},
				{
					outputDirectory: directory,
					readProfile: () => ({
						...value,
						entitlements: { ...value.entitlements, "com.apple.developer.associated-domains": "*" },
					}),
				},
			)
			expect(readFileSync(result.entitlementsPath, "utf8")).toContain(
				`<key>com.apple.developer.associated-domains</key><array><string>webcredentials:${policy.rpId}</string></array>`,
			)
			expect(JSON.parse(readFileSync(result.associationFile, "utf8"))).toEqual({
				webcredentials: { apps: ["PREFIX.network.hyperbridge.simplex"] },
			})
		} finally {
			rmSync(directory, { recursive: true, force: true })
		}
	})
	it("uses the authorized application prefix independently of the signing team", () => {
		expect(validateMacPasskeyProfile(profile(), policy)).toBe("PREFIX.network.hyperbridge.simplex")
	})
	it("rejects wrong team, expired, development, App Store and mismatched domain profiles", () => {
		const bad = [
			{ ...profile(), teams: ["OTHER"] },
			{ ...profile(), expiresAt: "2000-01-01" },
			{ ...profile(), entitlements: { ...profile().entitlements, "get-task-allow": true } },
			{ ...profile(), provisionsAllDevices: false },
			{
				...profile(),
				entitlements: {
					...profile().entitlements,
					"com.apple.developer.associated-domains": ["webcredentials:other.example"],
				},
			},
			{
				...profile(),
				entitlements: { ...profile().entitlements, "com.apple.application-identifier": "PREFIX.other.app" },
			},
		]
		for (const value of bad) expect(() => validateMacPasskeyProfile(value, policy)).toThrow()
	})
})
