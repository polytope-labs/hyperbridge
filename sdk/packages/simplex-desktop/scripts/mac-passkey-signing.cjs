const { spawnSync } = require("node:child_process")
const { mkdirSync, readFileSync, writeFileSync } = require("node:fs")
const { resolve } = require("node:path")
const passkeyConfiguration = require("../resources/passkeys.json")
const BUNDLE_ID = "network.hyperbridge.simplex"
const packageRoot = resolve(__dirname, "..")

function run(command, args, input) {
	const result = spawnSync(command, args, { encoding: "utf8", input })
	if (result.error || result.status !== 0) throw new Error(`Could not read Apple provisioning profile (${command})`)
	return result.stdout.trim()
}
function readProvisioningProfile(path) {
	const xml = run("security", ["cms", "-D", "-i", path])
	return {
		entitlements: JSON.parse(run("plutil", ["-extract", "Entitlements", "json", "-o", "-", "-"], xml)),
		teams: JSON.parse(run("plutil", ["-extract", "TeamIdentifier", "json", "-o", "-", "-"], xml)),
		platforms: JSON.parse(run("plutil", ["-extract", "Platform", "json", "-o", "-", "-"], xml)),
		provisionsAllDevices:
			run("plutil", ["-extract", "ProvisionsAllDevices", "raw", "-o", "-", "-"], xml) === "true",
		expiresAt: run("plutil", ["-extract", "ExpirationDate", "raw", "-o", "-", "-"], xml),
	}
}
function validateMacPasskeyProfile(profile, { rpId, teamId }, now = Date.now()) {
	if (typeof teamId !== "string" || !/^[A-Z0-9]{6,20}$/.test(teamId))
		throw new Error("APPLE_TEAM_ID must be a valid Apple signing team")
	if (
		typeof rpId !== "string" ||
		rpId.length > 253 ||
		rpId !== rpId.toLowerCase() ||
		!rpId.includes(".") ||
		!rpId.split(".").every((label) => /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(label)) ||
		/^\d+(\.\d+){3}$/.test(rpId)
	)
		throw new Error("SIMPLEX_PASSKEY_RP_ID must be a permanent DNS hostname")
	const e = profile.entitlements ?? {}
	const applicationIdentifier = e["com.apple.application-identifier"]
	if (
		!Array.isArray(profile.teams) ||
		!profile.teams.includes(teamId) ||
		e["com.apple.developer.team-identifier"] !== teamId
	)
		throw new Error("Apple profile signing team does not match APPLE_TEAM_ID")
	if (
		typeof applicationIdentifier !== "string" ||
		!new RegExp(`^[A-Z0-9]+\\.${BUNDLE_ID.replaceAll(".", "\\.")}$`).test(applicationIdentifier)
	)
		throw new Error("Apple profile must authorize the exact Simplex application identifier")
	const expiry = Date.parse(profile.expiresAt)
	if (!Number.isFinite(expiry) || expiry <= now) throw new Error("Apple provisioning profile is expired or invalid")
	if (e["get-task-allow"] === true)
		throw new Error("Release builds require a Developer ID distribution profile, not a development profile")
	if (!profile.provisionsAllDevices || !profile.platforms?.includes("OSX"))
		throw new Error("Native passkey releases require a macOS Developer ID distribution profile")
	// A Developer ID profile grants every domain as the plain string "*", not a list.
	// The app's generated entitlement still contains only its explicit webcredentials RP.
	const domains = e["com.apple.developer.associated-domains"]
	const authorized =
		domains === "*" ||
		(Array.isArray(domains) && (domains.includes("*") || domains.includes(`webcredentials:${rpId}`)))
	if (!authorized) throw new Error("Apple profile does not authorize the passkey associated domain")
	return applicationIdentifier
}
function prepareMacPasskeySigning(
	env,
	{ readProfile = readProvisioningProfile, outputDirectory = resolve(packageRoot, "build", "mac-passkeys") } = {},
) {
	const profilePath = env.SIMPLEX_MACOS_PROVISIONING_PROFILE_PATH
	const rpId = env.SIMPLEX_PASSKEY_RP_ID || passkeyConfiguration.rpId
	if (!profilePath) throw new Error("Signed macOS native passkeys require SIMPLEX_MACOS_PROVISIONING_PROFILE_PATH")
	const applicationIdentifier = validateMacPasskeyProfile(readProfile(profilePath), {
		rpId,
		teamId: env.APPLE_TEAM_ID,
	})
	const directory = resolve(outputDirectory)
	mkdirSync(directory, { recursive: true })
	const base = readFileSync(resolve(packageRoot, "resources", "entitlements.mac.plist"), "utf8")
	const additions = `<key>com.apple.application-identifier</key><string>${applicationIdentifier}</string>\n<key>com.apple.developer.team-identifier</key><string>${env.APPLE_TEAM_ID}</string>\n<key>com.apple.developer.associated-domains</key><array><string>webcredentials:${rpId}</string></array>\n`
	const entitlementsPath = resolve(directory, "entitlements.mac.plist")
	writeFileSync(entitlementsPath, base.replace("</dict>", `${additions}</dict>`))
	writeFileSync(
		resolve(directory, "apple-app-site-association"),
		JSON.stringify({ webcredentials: { apps: [applicationIdentifier] } }, null, 2) + "\n",
	)
	return {
		profilePath: resolve(profilePath),
		entitlementsPath,
		associationFile: resolve(directory, "apple-app-site-association"),
		applicationIdentifier,
		rpId,
	}
}
module.exports = { readProvisioningProfile, validateMacPasskeyProfile, prepareMacPasskeySigning }
