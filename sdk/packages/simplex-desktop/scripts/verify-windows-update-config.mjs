import { readFile, readdir } from "node:fs/promises"
import { join } from "node:path"
import { fileURLToPath } from "node:url"
import { parse } from "yaml"

export function assertWindowsUpdatePublisher(contents, expectedPublisher) {
	if (!expectedPublisher?.trim()) throw new Error("Expected Windows publisher is required")
	const config = parse(contents)
	const value = config?.publisherName
	const publishers = (Array.isArray(value) ? value : [value]).filter(
		(name) => typeof name === "string" && name.trim().length > 0,
	)
	if (!publishers.includes(expectedPublisher)) {
		throw new Error(
			`Packaged app-update.yml must name the expected Windows publisher ${JSON.stringify(expectedPublisher)}`,
		)
	}
	return publishers
}

export async function verifyWindowsUpdateConfig(root, expectedPublisher) {
	const unpacked = (await readdir(root, { withFileTypes: true }))
		.filter((entry) => entry.isDirectory() && entry.name.startsWith("win") && entry.name.endsWith("-unpacked"))
		.map((entry) => entry.name)
	if (unpacked.length !== 1) {
		throw new Error(`Expected one unpacked Windows application under ${root}, found ${unpacked.length}`)
	}
	const configPath = join(root, unpacked[0], "resources", "app-update.yml")
	assertWindowsUpdatePublisher(await readFile(configPath, "utf8"), expectedPublisher)
	return configPath
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const [root, expectedPublisher] = process.argv.slice(2)
	if (!root || !expectedPublisher) {
		throw new Error("Usage: verify-windows-update-config.mjs <release-root> <expected-publisher>")
	}
	await verifyWindowsUpdateConfig(root, expectedPublisher)
}
