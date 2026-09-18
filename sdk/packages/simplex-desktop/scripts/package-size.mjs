import { lstat, readdir } from "node:fs/promises"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

async function directoryBytes(path) {
	let total = 0
	for (const entry of await readdir(path, { withFileTypes: true })) {
		const child = join(path, entry.name)
		if (entry.isDirectory()) total += await directoryBytes(child)
		else if (entry.isFile()) total += (await lstat(child)).size
	}
	return total
}

export async function installedAppDirectories(outputRoot) {
	const results = []
	for (const entry of await readdir(outputRoot, { withFileTypes: true })) {
		if (!entry.isDirectory()) continue
		const path = join(outputRoot, entry.name)
		if (entry.name.endsWith("-unpacked")) {
			results.push(path)
			continue
		}
		for (const nested of await readdir(path, { withFileTypes: true })) {
			if (nested.isDirectory() && nested.name.endsWith(".app")) results.push(join(path, nested.name))
		}
	}
	return results
}

export async function assertInstalledSize(outputRoot, budgetMiB) {
	const apps = await installedAppDirectories(outputRoot)
	if (apps.length === 0) throw new Error(`No unpacked desktop application found under ${outputRoot}`)
	const budget = budgetMiB * 1024 * 1024
	const measured = []
	for (const app of apps) {
		const bytes = await directoryBytes(app)
		if (bytes > budget) {
			throw new Error(`${app} is ${(bytes / 1024 / 1024).toFixed(1)} MiB; budget is ${budgetMiB} MiB`)
		}
		measured.push({ app, bytes })
	}
	return measured
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const rootIndex = process.argv.indexOf("--root")
	const budgetIndex = process.argv.indexOf("--budget-mib")
	const outputRoot = resolve(rootIndex === -1 ? "release" : process.argv[rootIndex + 1])
	const budgetMiB = Number(budgetIndex === -1 ? 520 : process.argv[budgetIndex + 1])
	assertInstalledSize(outputRoot, budgetMiB)
		.then((results) => {
			for (const { app, bytes } of results) {
				process.stdout.write(`${app}: ${(bytes / 1024 / 1024).toFixed(1)} MiB (budget ${budgetMiB} MiB)\n`)
			}
		})
		.catch((error) => {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
			process.exitCode = 1
		})
}
