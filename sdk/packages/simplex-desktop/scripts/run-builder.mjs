import { spawnSync } from "node:child_process"
import { accessSync, readFileSync } from "node:fs"
import { dirname, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const cli = resolve(packageRoot, "tooling/node_modules/electron-builder/cli.js")
const packageVersion = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8")).version
const wait = (milliseconds) => new Promise((resolveWait) => setTimeout(resolveWait, milliseconds))

export function normalizeBuilderArguments(args) {
	const delimiter = args.indexOf("--")
	return delimiter === -1 ? args : [...args.slice(0, delimiter), ...args.slice(delimiter + 1)]
}

export function builderArguments(args, version = packageVersion) {
	const normalized = normalizeBuilderArguments(args)
	if (!version.includes("-beta.")) return normalized
	if (normalized.some((argument) => /^(?:-c|--config)\.publish\.channel=/.test(argument))) return normalized
	return [...normalized, "-c.publish.channel=beta"]
}

export function assertPackagingNodeVersion(version = process.versions.node) {
	if (Number(version.split(".")[0]) < 24) {
		throw new Error(`Simplex desktop packaging requires Node 24 or newer; found ${version}`)
	}
}

export async function runBuilderWithRetries(execute, options = {}) {
	const attempts = options.attempts ?? 1
	const retryDelayMs = options.retryDelayMs ?? 5_000
	const sleep = options.sleep ?? wait
	if (!Number.isInteger(attempts) || attempts < 1) throw new Error("Packaging attempts must be a positive integer")
	for (let attempt = 1; attempt <= attempts; attempt += 1) {
		const status = await execute()
		if (status === 0 || attempt === attempts) return status
		process.stderr.write(`Desktop packaging attempt ${attempt} failed; retrying in ${retryDelayMs}ms\n`)
		await sleep(retryDelayMs)
	}
	return 1
}

export async function runBuilder(args, spawn = spawnSync) {
	assertPackagingNodeVersion()
	try {
		accessSync(cli)
	} catch {
		throw new Error("Desktop packaging tools are missing; run `pnpm --dir tooling install --frozen-lockfile` first")
	}
	const attempts = Number.parseInt(process.env.SIMPLEX_DESKTOP_PACKAGE_ATTEMPTS ?? "1", 10)
	return runBuilderWithRetries(
		() => {
			const result = spawn(process.execPath, [cli, ...builderArguments(args)], {
				cwd: packageRoot,
				stdio: "inherit",
			})
			if (result.error) throw result.error
			return result.status ?? 1
		},
		{ attempts },
	)
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
	process.exitCode = await runBuilder(process.argv.slice(2))
}
