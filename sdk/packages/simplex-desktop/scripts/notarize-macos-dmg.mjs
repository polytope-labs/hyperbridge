import { spawnSync } from "node:child_process"
import { readdir } from "node:fs/promises"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

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
 * Staples the notarization ticket to the DMG. The ticket comes from Apple's
 * CloudKit endpoint, which sometimes drops the connection, so a failed
 * attempt is retried after a pause.
 */
export async function stapleDmg(dmg, { attempts = 5, delayMs = 30_000, runCommand = run, sleep } = {}) {
	const pause = sleep ?? ((ms) => new Promise((resolve) => setTimeout(resolve, ms)))
	for (let attempt = 1; ; attempt++) {
		try {
			return runCommand("xcrun", ["stapler", "staple", "-v", dmg])
		} catch (error) {
			if (attempt >= attempts) throw error
			process.stderr.write(`Stapling attempt ${attempt} of ${attempts} failed; retrying in ${delayMs / 1000}s\n`)
			await pause(delayMs)
		}
	}
}

async function findDmg(root) {
	const matches = (await readdir(root, { withFileTypes: true }))
		.filter((entry) => entry.isFile() && entry.name.endsWith(".dmg"))
		.map((entry) => join(root, entry.name))
	if (matches.length !== 1) throw new Error(`Expected one macOS DMG under ${root}, found ${matches.length}`)
	return matches[0]
}

export async function notarizeMacDmg(root, env = process.env) {
	const dmg = await findDmg(resolve(root))
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
	return { dmg, submissionId: result.id }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const { dmg, submissionId } = await notarizeMacDmg(process.argv[2] ?? "release")
	process.stdout.write(`Notarized and stapled ${dmg} (${submissionId})\n`)
}
