import { spawnSync } from "node:child_process"
import { createHash } from "node:crypto"
import { createReadStream } from "node:fs"
import { readFile, readdir, stat, writeFile } from "node:fs/promises"
import { basename, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { parse, stringify } from "yaml"

function run(command, args) {
	const result = spawnSync(command, args, { encoding: "utf8" })
	if (result.error) throw result.error
	if (result.status !== 0) {
		throw new Error(`${command} ${args.join(" ")} failed:\n${result.stdout}${result.stderr}`)
	}
	return result.stdout
}

function requiredEnvironment(env, name) {
	const value = env[name]?.trim()
	if (!value) throw new Error(`macOS DMG notarization requires ${name}`)
	return value
}

export function notarytoolArguments(dmg, env = process.env) {
	return [
		"notarytool",
		"submit",
		dmg,
		"--key",
		requiredEnvironment(env, "APPLE_API_KEY"),
		"--key-id",
		requiredEnvironment(env, "APPLE_API_KEY_ID"),
		"--issuer",
		requiredEnvironment(env, "APPLE_API_ISSUER"),
		"--wait",
		"--output-format",
		"json",
	]
}

/**
 * Runs `action` again after a pause when it throws. Apple's ticket server
 * sometimes drops connections from CI runners, which fails stapling,
 * Gatekeeper assessment and ticket validation alike.
 */
export async function retryAppleNetwork(label, action, { attempts = 5, delayMs = 30_000, sleep } = {}) {
	const pause = sleep ?? ((ms) => new Promise((resolve) => setTimeout(resolve, ms)))
	for (let attempt = 1; ; attempt++) {
		try {
			return action()
		} catch (error) {
			if (attempt >= attempts) throw error
			process.stderr.write(`${label} attempt ${attempt} of ${attempts} failed; retrying in ${delayMs / 1000}s\n`)
			await pause(delayMs)
		}
	}
}

/** Staples the notarization ticket to the DMG. */
export function stapleDmg(dmg, { attempts, delayMs, runCommand = run, sleep } = {}) {
	return retryAppleNetwork("Stapling", () => runCommand("xcrun", ["stapler", "staple", "-v", dmg]), {
		attempts,
		delayMs,
		sleep,
	})
}

async function sha512(path) {
	const hash = createHash("sha512")
	for await (const chunk of createReadStream(path)) hash.update(chunk)
	return hash.digest("base64")
}

/**
 * Stapling writes the ticket into the DMG, so the hash and size electron-builder
 * recorded for it in the updater metadata no longer match. Rewrites the DMG's
 * entry in every metadata file under `root` that lists it.
 */
export async function refreshUpdaterMetadata(root, dmg) {
	const name = basename(dmg)
	const digest = await sha512(dmg)
	const { size } = await stat(dmg)
	const updated = []
	for (const entry of (await readdir(root)).filter((file) => file.endsWith(".yml"))) {
		const path = join(root, entry)
		const metadata = parse(await readFile(path, "utf8"))
		const listed = (metadata?.files ?? []).filter((file) => file?.url === name)
		for (const file of listed) Object.assign(file, { sha512: digest, size })
		if (metadata?.path === name) metadata.sha512 = digest
		if (listed.length === 0 && metadata?.path !== name) continue
		await writeFile(path, stringify(metadata))
		updated.push(entry)
	}
	return updated
}

async function findDmg(root) {
	const matches = (await readdir(root, { withFileTypes: true }))
		.filter((entry) => entry.isFile() && entry.name.endsWith(".dmg"))
		.map((entry) => join(root, entry.name))
	if (matches.length !== 1) throw new Error(`Expected one macOS DMG under ${root}, found ${matches.length}`)
	return matches[0]
}

export async function notarizeMacDmg(root, env = process.env) {
	const directory = resolve(root)
	const dmg = await findDmg(directory)
	const output = run("xcrun", notarytoolArguments(dmg, env))
	let result
	try {
		result = JSON.parse(output)
	} catch {
		throw new Error(`notarytool returned invalid JSON: ${output}`)
	}
	if (result.status !== "Accepted") {
		throw new Error(`Apple rejected ${dmg}: ${result.status ?? "unknown status"} (${result.id ?? "no id"})`)
	}
	await stapleDmg(dmg)
	await refreshUpdaterMetadata(directory, dmg)
	return { dmg, submissionId: result.id }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const { dmg, submissionId } = await notarizeMacDmg(process.argv[2] ?? "release")
	process.stdout.write(`Notarized and stapled ${dmg} (${submissionId})\n`)
}
