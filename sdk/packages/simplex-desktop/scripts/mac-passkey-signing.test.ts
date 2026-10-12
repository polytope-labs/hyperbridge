import { createRequire } from "node:module"
import { describe, expect, it } from "vitest"
const { validateMacPasskeyProfile } = createRequire(import.meta.url)("./mac-passkey-signing.cjs")
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
