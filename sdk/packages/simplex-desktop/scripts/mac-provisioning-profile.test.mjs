import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs"
import { createRequire } from "node:module"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"

const { readProvisioningProfile, validateMacPasskeyProfile, prepareMacPasskeySigning } = createRequire(import.meta.url)(
	"./mac-passkey-signing.cjs",
)
const policy = { rpId: "passkeys.example.com", teamId: "TEAM123" }

function run(command, args) {
	const result = spawnSync(command, args, { encoding: "utf8" })
	assert.equal(result.status, 0, `${command} failed: ${result.stderr}`)
}

function fixture(directory, domains) {
	const xml = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CreationDate</key><date>2026-01-01T00:00:00Z</date>
<key>ExpirationDate</key><date>2099-01-01T00:00:00Z</date>
<key>DeveloperCertificates</key><array><data>AQID</data></array>
<key>DER-Encoded-Profile</key><data>AQID</data>
<key>TeamIdentifier</key><array><string>TEAM123</string></array>
<key>Platform</key><array><string>OSX</string></array>
<key>ProvisionsAllDevices</key><true/>
<key>Entitlements</key><dict>
<key>com.apple.application-identifier</key><string>PREFIX.network.hyperbridge.simplex</string>
<key>com.apple.developer.team-identifier</key><string>TEAM123</string>
<key>com.apple.developer.associated-domains</key>${domains}
<key>get-task-allow</key><false/>
</dict></dict></plist>`
	const payload = join(directory, "profile.plist")
	const path = join(directory, "profile.provisionprofile")
	writeFileSync(payload, xml)
	run("/usr/bin/openssl", [
		"smime",
		"-sign",
		"-binary",
		"-nodetach",
		"-in",
		payload,
		"-signer",
		join(directory, "certificate.pem"),
		"-inkey",
		join(directory, "key.pem"),
		"-outform",
		"DER",
		"-out",
		path,
	])
	return path
}

test(
	"reads CMS profiles with binary/date metadata and prepares domain-scoped Mac signing",
	{
		skip: process.platform !== "darwin",
	},
	() => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-native-profile-"))
		try {
			run("/usr/bin/openssl", [
				"req",
				"-x509",
				"-newkey",
				"rsa:2048",
				"-nodes",
				"-days",
				"1",
				"-subj",
				"/CN=Simplex provisioning test fixture",
				"-keyout",
				join(directory, "key.pem"),
				"-out",
				join(directory, "certificate.pem"),
			])
			for (const domains of [
				"<string>*</string>",
				"<array><string>*</string></array>",
				`<array><string>webcredentials:${policy.rpId}</string></array>`,
			]) {
				const path = fixture(directory, domains)
				const profile = readProvisioningProfile(path)
				assert.equal(validateMacPasskeyProfile(profile, policy), "PREFIX.network.hyperbridge.simplex")
				assert.deepEqual(profile.teams, [policy.teamId])
				assert.deepEqual(profile.platforms, ["OSX"])
				assert.equal(profile.provisionsAllDevices, true)
				assert.ok(Date.parse(profile.expiresAt) > Date.now())
				const result = prepareMacPasskeySigning(
					{
						APPLE_TEAM_ID: policy.teamId,
						SIMPLEX_PASSKEY_RP_ID: policy.rpId,
						SIMPLEX_MACOS_PROVISIONING_PROFILE_PATH: path,
					},
					{ outputDirectory: join(directory, "generated") },
				)
				assert.ok(
					readFileSync(result.entitlementsPath, "utf8").includes(
						`<key>com.apple.developer.associated-domains</key><array><string>webcredentials:${policy.rpId}</string></array>`,
					),
				)
				assert.deepEqual(JSON.parse(readFileSync(result.associationFile, "utf8")), {
					webcredentials: { apps: ["PREFIX.network.hyperbridge.simplex"] },
				})
			}
		} finally {
			rmSync(directory, { recursive: true, force: true })
		}
	},
)
