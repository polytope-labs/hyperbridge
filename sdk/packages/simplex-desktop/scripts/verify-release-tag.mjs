import { readFile } from "node:fs/promises"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")

export async function verifyReleaseTag(tag) {
	const manifest = JSON.parse(await readFile(join(packageRoot, "package.json"), "utf8"))
	const expected = `simplex-desktop-v${manifest.version}`
	if (tag !== expected) throw new Error(`Desktop release tag ${tag} must exactly match ${expected}`)
	if (!/^simplex-desktop-v\d+\.\d+\.\d+(?:-beta\.\d+)?$/.test(tag)) {
		throw new Error(`Desktop release tag has an unsupported stable/beta form: ${tag}`)
	}
	return manifest.version
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	verifyReleaseTag(process.argv[2] ?? process.env.GITHUB_REF_NAME ?? "")
		.then((version) => process.stdout.write(`Verified Simplex desktop release v${version}\n`))
		.catch((error) => {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
			process.exitCode = 1
		})
}
