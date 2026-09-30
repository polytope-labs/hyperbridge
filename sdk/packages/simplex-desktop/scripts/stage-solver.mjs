import { spawnSync } from "node:child_process"
import { access, cp, mkdir, readFile, rm } from "node:fs/promises"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { EXTERNAL_RUNTIME_PACKAGES } from "./package-layout.mjs"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const workspaceRoot = resolve(packageRoot, "../..")
const simplexRoot = resolve(packageRoot, "../simplex")
const output = resolve(packageRoot, "build/package")
const deployOutput = join(output, "runtime-deps")

function runPnpm(args) {
	const pnpmScript = process.env.npm_execpath
	const command = pnpmScript ? process.execPath : process.platform === "win32" ? "pnpm.cmd" : "pnpm"
	const commandArgs = pnpmScript ? [pnpmScript, ...args] : args
	const result = spawnSync(command, commandArgs, { cwd: workspaceRoot, stdio: "inherit" })
	if (result.error) throw result.error
	if (result.status !== 0) throw new Error(`Runtime dependency install failed with exit code ${result.status}`)
}

export async function stageSolverDistribution(sourceRoot, destinationRoot) {
	const solver = join(sourceRoot, "dist/bin/simplex.js")
	const sourceMap = join(sourceRoot, "dist/bin/simplex.js.map")
	const ui = join(sourceRoot, "dist/ui")
	const manifest = join(sourceRoot, "package.json")
	await Promise.all([access(solver), access(sourceMap), access(join(ui, "index.html")), access(manifest)])
	const parsedSourceMap = JSON.parse(await readFile(sourceMap, "utf8"))
	if (Object.hasOwn(parsedSourceMap, "sourcesContent")) {
		throw new Error(`${sourceMap} embeds source content; rebuild Simplex with sourcesContent disabled`)
	}

	await mkdir(join(destinationRoot, "dist/bin"), { recursive: true })
	await Promise.all([
		cp(solver, join(destinationRoot, "dist/bin/simplex.js")),
		cp(sourceMap, join(destinationRoot, "dist/bin/simplex.js.map")),
		cp(ui, join(destinationRoot, "dist/ui"), { recursive: true }),
		cp(manifest, join(destinationRoot, "package.json")),
	])
}

export async function stageSolverResources() {
	await rm(output, { recursive: true, force: true })
	await stageSolverDistribution(simplexRoot, join(output, "simplex"))
	await mkdir(deployOutput, { recursive: true })
	await Promise.all([
		cp(join(packageRoot, "runtime/package.json"), join(deployOutput, "package.json")),
		cp(join(packageRoot, "runtime/pnpm-lock.yaml"), join(deployOutput, "pnpm-lock.yaml")),
	])
	runPnpm([
		"--dir",
		deployOutput,
		"install",
		"--prod",
		"--frozen-lockfile",
		"--prefer-offline",
		"--ignore-workspace",
		"--ignore-scripts",
		"--config.auto-install-peers=false",
		// The frozen runtime-only lock controls every version. Prefer the shared
		// pnpm store while still allowing a clean CI runner to fetch missing tarballs.
		"--config.node-linker=hoisted",
	])

	for (const name of EXTERNAL_RUNTIME_PACKAGES) {
		const manifest = join(deployOutput, "node_modules", name, "package.json")
		const parsed = JSON.parse(await readFile(manifest, "utf8"))
		if (parsed.name !== name)
			throw new Error(`Packaged runtime dependency ${manifest} identifies as ${parsed.name}`)
	}
	return output
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	stageSolverResources()
		.then((path) => process.stdout.write(`Staged packaged solver resources at ${path}\n`))
		.catch((error) => {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
			process.exitCode = 1
		})
}
