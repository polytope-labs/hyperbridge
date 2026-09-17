import { cp, mkdir, readFile, readdir, writeFile } from "node:fs/promises"
import { basename, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { parse, stringify } from "yaml"

export const parseUpdateMetadata = parse
export const stringifyUpdateMetadata = stringify

async function regularFiles(directory) {
	const files = []
	const entries = await readdir(directory, { withFileTypes: true })
	entries.sort((left, right) => left.name.localeCompare(right.name))
	for (const entry of entries) {
		const path = join(directory, entry.name)
		if (entry.isDirectory()) files.push(...(await regularFiles(path)))
		else if (entry.isFile()) files.push(path)
	}
	return files
}

export function mergeUpdateMetadata(documents) {
	if (documents.length === 0) throw new Error("No update metadata documents were provided")
	const [first, ...rest] = documents
	for (const document of rest) {
		if (document.version !== first.version) {
			throw new Error(`Cannot merge updater metadata for ${first.version} and ${document.version}`)
		}
	}
	const files = documents.flatMap((document) => document.files ?? [])
	const urls = new Set()
	for (const file of files) {
		if (!file?.url || urls.has(file.url)) throw new Error(`Duplicate or missing updater artifact URL: ${file?.url}`)
		urls.add(file.url)
	}
	return { ...first, files }
}

export async function assembleRelease(inputRoot, outputRoot) {
	await mkdir(outputRoot, { recursive: true })
	const metadata = new Map()
	const outputs = new Set()
	for (const source of await regularFiles(inputRoot)) {
		const name = basename(source)
		if (name.endsWith(".yml")) {
			const group = metadata.get(name) ?? []
			group.push(parse(await readFile(source, "utf8")))
			metadata.set(name, group)
			continue
		}
		if (outputs.has(name)) throw new Error(`Duplicate release artifact: ${name}`)
		outputs.add(name)
		await cp(source, join(outputRoot, name))
	}
	for (const [name, documents] of metadata) {
		const merged = mergeUpdateMetadata(documents)
		await writeFile(join(outputRoot, name), stringify(merged))
		outputs.add(name)
	}
	return [...outputs].sort()
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const input = resolve(process.argv[2] ?? "release-input")
	const output = resolve(process.argv[3] ?? "release-assets")
	assembleRelease(input, output)
		.then((files) => process.stdout.write(`Assembled ${files.length} release assets in ${output}\n`))
		.catch((error) => {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
			process.exitCode = 1
		})
}
