import { spawnSync } from "node:child_process"
import { accessSync, readFileSync } from "node:fs"
import { dirname, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const cli = resolve(packageRoot, "tooling/node_modules/electron-builder/cli.js")
const packageVersion = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8")).version

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

export function runBuilder(args, spawn = spawnSync) {
	assertPackagingNodeVersion()
	try {
		accessSync(cli)
	} catch {
		throw new Error("Desktop packaging tools are missing; run `pnpm --dir tooling install --frozen-lockfile` first")
	}
	const result = spawn(process.execPath, [cli, ...builderArguments(args)], {
		cwd: packageRoot,
		stdio: "inherit",
	})
	if (result.error) throw result.error
	return result.status ?? 1
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
	process.exitCode = runBuilder(process.argv.slice(2))
}
