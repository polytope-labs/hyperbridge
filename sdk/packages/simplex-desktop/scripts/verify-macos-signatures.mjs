import { spawnSync } from "node:child_process"
import { readdir } from "node:fs/promises"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { createRequire } from "node:module"
import { retryAppleNetwork } from "./notarize-macos-dmg.mjs"

const require = createRequire(import.meta.url)
const passkeyConfiguration = require("../resources/passkeys.json")
const { readProvisioningProfile, validateMacPasskeyProfile } = require("./mac-passkey-signing.cjs")

const REQUIRED_ENTITLEMENTS = [
	"com.apple.security.cs.allow-jit",
	"com.apple.security.cs.allow-unsigned-executable-memory",
]

function run(command, args) {
	const result = spawnSync(command, args, { encoding: "utf8" })
	if (result.error) throw result.error
	if (result.status !== 0) {
		throw new Error(`${command} ${args.join(" ")} failed:\n${result.stdout}${result.stderr}`)
	}
	return `${result.stdout}${result.stderr}`
}

export function teamIdentifier(output) {
	const match = /^TeamIdentifier=(.+)$/m.exec(output)
	if (!match) throw new Error("Code signature has no TeamIdentifier")
	return match[1].trim()
}

export function entitlementKeys(output) {
	return [...output.matchAll(/<key>([^<]+)<\/key>/g)].map((match) => match[1]).sort()
}

function verifyTeam(path, expectedTeamId) {
	const details = run("codesign", ["--display", "--verbose=4", path])
	const actualTeamId = teamIdentifier(details)
	if (actualTeamId !== expectedTeamId) {
		throw new Error(`${path} is signed by TeamIdentifier ${actualTeamId}, expected ${expectedTeamId}`)
	}
}

function verifyIdentity(path, expectedTeamId, identity) {
	verifyTeam(path, expectedTeamId)
	const entitlements = entitlementKeys(run("codesign", ["--display", "--entitlements", ":-", path]))
	const expected = identity
		? [
				...REQUIRED_ENTITLEMENTS,
				"com.apple.application-identifier",
				"com.apple.developer.team-identifier",
				"com.apple.developer.associated-domains",
			]
		: REQUIRED_ENTITLEMENTS
	if (JSON.stringify(entitlements) !== JSON.stringify([...expected].sort())) {
		throw new Error(`${path} has unexpected entitlements: ${entitlements.join(", ") || "none"}`)
	}
	if (identity) {
		const xml = run("codesign", ["--display", "--entitlements", ":-", path])
		const parsed = spawnSync("plutil", ["-convert", "json", "-o", "-", "-"], {
			input: xml.slice(xml.indexOf("<?xml")),
			encoding: "utf8",
		})
		if (parsed.status !== 0) throw new Error("Could not parse main application entitlements")
		const values = JSON.parse(parsed.stdout)
		if (
			values["com.apple.application-identifier"] !== identity.applicationIdentifier ||
			values["com.apple.developer.team-identifier"] !== expectedTeamId ||
			JSON.stringify(values["com.apple.developer.associated-domains"]) !==
				JSON.stringify([`webcredentials:${identity.rpId}`])
		)
			throw new Error("Main application passkey identity/domain entitlements do not match release configuration")
	}
}

async function findMacApp(root) {
	for (const entry of await readdir(root, { withFileTypes: true })) {
		if (!entry.isDirectory() || !entry.name.startsWith("mac")) continue
		const app = join(root, entry.name, "Simplex.app")
		try {
			await readdir(app)
			return app
		} catch {
			// Try the next architecture-specific unpacked directory.
		}
	}
	throw new Error(`No unpacked Simplex.app found under ${root}`)
}

async function findMacDmg(root) {
	const matches = (await readdir(root, { withFileTypes: true }))
		.filter((entry) => entry.isFile() && entry.name.endsWith(".dmg"))
		.map((entry) => join(root, entry.name))
	if (matches.length !== 1) {
		throw new Error(`Expected one macOS DMG under ${root}, found ${matches.length}`)
	}
	return matches[0]
}

async function findHelperApps(app) {
	const frameworks = join(app, "Contents", "Frameworks")
	const helpers = (await readdir(frameworks, { withFileTypes: true }))
		.filter((entry) => entry.isDirectory() && / Helper(?: \([^)]+\))?\.app$/.test(entry.name))
		.map((entry) => join(frameworks, entry.name))
	if (helpers.length === 0) throw new Error(`No Electron helper applications found under ${frameworks}`)
	return helpers
}

export async function verifyMacSignatures(root, expectedTeamId) {
	if (!expectedTeamId?.trim()) throw new Error("Expected Apple team ID is required")
	const releaseRoot = resolve(root)
	const app = await findMacApp(releaseRoot)
	const dmg = await findMacDmg(releaseRoot)
	const helpers = await findHelperApps(app)
	const runtime = join(app, "Contents", "Resources", "runtime", "node")

	run("codesign", ["--verify", "--deep", "--strict", "--verbose=4", app])
	run("codesign", ["--verify", "--strict", "--verbose=4", runtime])
	run("codesign", ["--verify", "--strict", "--verbose=4", dmg])
	const rpId = process.env.SIMPLEX_PASSKEY_RP_ID || passkeyConfiguration.rpId
	const applicationIdentifier = validateMacPasskeyProfile(
		readProvisioningProfile(join(app, "Contents", "embedded.provisionprofile")),
		{ rpId, teamId: expectedTeamId },
	)
	verifyIdentity(app, expectedTeamId, { rpId, applicationIdentifier })
	for (const helper of helpers) verifyIdentity(helper, expectedTeamId)
	verifyIdentity(runtime, expectedTeamId)
	const native = join(app, "Contents", "Resources", "desktop", "native", "mac-passkeys.node")
	run("codesign", ["--verify", "--strict", "--verbose=4", native])
	verifyTeam(native, expectedTeamId)
	const nativeEntitlements = entitlementKeys(run("codesign", ["--display", "--entitlements", ":-", native]))
	if (nativeEntitlements.some((key) => !REQUIRED_ENTITLEMENTS.includes(key)))
		throw new Error("Native library contains unexpected app identity entitlements")
	verifyTeam(dmg, expectedTeamId)
	// Gatekeeper and the stapler both look the ticket up at Apple, which drops CI connections now and then.
	await retryAppleNetwork("Gatekeeper assessment", () =>
		run("spctl", ["--assess", "--type", "execute", "--verbose=4", app]),
	)
	await retryAppleNetwork("Gatekeeper assessment", () =>
		run("spctl", ["--assess", "--type", "open", "--context", "context:primary-signature", "--verbose=4", dmg]),
	)
	await retryAppleNetwork("Ticket validation", () => run("xcrun", ["stapler", "validate", app]))
	await retryAppleNetwork("Ticket validation", () => run("xcrun", ["stapler", "validate", dmg]))
	return { app, dmg, helpers, runtime }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const root = process.argv[2] ?? "release"
	const expectedTeamId = process.argv[3]
	await verifyMacSignatures(root, expectedTeamId)
	process.stdout.write(
		"Verified macOS app, helpers, runtime, and DMG signatures, entitlements, Gatekeeper assessments, and notarization tickets\n",
	)
}
