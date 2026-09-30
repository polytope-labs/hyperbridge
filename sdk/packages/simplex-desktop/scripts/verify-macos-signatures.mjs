import { spawnSync } from "node:child_process"
import { readdir } from "node:fs/promises"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

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

function verifyIdentity(path, expectedTeamId) {
	verifyTeam(path, expectedTeamId)
	const entitlements = entitlementKeys(run("codesign", ["--display", "--entitlements", ":-", path]))
	if (JSON.stringify(entitlements) !== JSON.stringify([...REQUIRED_ENTITLEMENTS].sort())) {
		throw new Error(`${path} has unexpected entitlements: ${entitlements.join(", ") || "none"}`)
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
	verifyIdentity(app, expectedTeamId)
	for (const helper of helpers) verifyIdentity(helper, expectedTeamId)
	verifyIdentity(runtime, expectedTeamId)
	verifyTeam(dmg, expectedTeamId)
	run("spctl", ["--assess", "--type", "execute", "--verbose=4", app])
	run("spctl", ["--assess", "--type", "open", "--context", "context:primary-signature", "--verbose=4", dmg])
	run("xcrun", ["stapler", "validate", app])
	run("xcrun", ["stapler", "validate", dmg])
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
