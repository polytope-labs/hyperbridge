import { createHash } from "node:crypto"
import { createReadStream } from "node:fs"
import { access, readFile, readdir } from "node:fs/promises"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { parse } from "yaml"

async function sha512(path) {
	const hash = createHash("sha512")
	for await (const chunk of createReadStream(path)) hash.update(chunk)
	return hash.digest("base64")
}

export async function assertReleaseAssets(directory, version) {
	const channelName = version.includes("-beta.") ? "beta" : "latest"
	const { required, expectedUpdaterArtifacts } = releaseAssetContract(version, channelName)
	for (const name of required) await access(join(directory, name))

	const available = new Set(await readdir(directory))
	const checksums = new Map()
	for (const metadataName of required.filter((name) => name.endsWith(".yml"))) {
		const metadata = parse(await readFile(join(directory, metadataName), "utf8"))
		if (metadata.version !== version)
			throw new Error(`${metadataName} describes ${metadata.version}, expected ${version}`)
		if (!Array.isArray(metadata.files) || metadata.files.length === 0) {
			throw new Error(`${metadataName} has no update artifacts`)
		}
		const urls = new Set(metadata.files.map((file) => file.url))
		for (const expected of expectedUpdaterArtifacts.get(metadataName) ?? []) {
			if (!urls.has(expected)) throw new Error(`${metadataName} does not reference ${expected}`)
		}
		for (const file of metadata.files) {
			if (!file.sha512) throw new Error(`${metadataName} lacks SHA-512 for ${file.url}`)
			if (!available.has(file.url))
				throw new Error(`${metadataName} references missing release artifact ${file.url}`)
			let actual = checksums.get(file.url)
			if (!actual) {
				actual = await sha512(join(directory, file.url))
				checksums.set(file.url, actual)
			}
			if (actual !== file.sha512) {
				throw new Error(`${metadataName} has an invalid SHA-512 for ${file.url}`)
			}
		}
	}
}

export function releaseAssetContract(version, channelName = version.includes("-beta.") ? "beta" : "latest") {
	const artifacts = [
		`Simplex-${version}-mac-arm64.dmg`,
		`Simplex-${version}-mac-arm64.zip`,
		`Simplex-${version}-mac-x64.dmg`,
		`Simplex-${version}-mac-x64.zip`,
		`Simplex-${version}-win-x64.exe`,
		`Simplex-${version}-linux-x86_64.AppImage`,
		`Simplex-${version}-linux-amd64.deb`,
		`Simplex-${version}-linux-arm64.AppImage`,
		`Simplex-${version}-linux-arm64.deb`,
		`Simplex-${version}-mac-arm64.zip.blockmap`,
		`Simplex-${version}-mac-x64.zip.blockmap`,
		`Simplex-${version}-win-x64.exe.blockmap`,
	]
	const metadata = [
		`${channelName}.yml`,
		`${channelName}-mac.yml`,
		`${channelName}-linux.yml`,
		`${channelName}-linux-arm64.yml`,
	]
	const expectedUpdaterArtifacts = new Map([
		[`${channelName}.yml`, [`Simplex-${version}-win-x64.exe`]],
		[`${channelName}-mac.yml`, [`Simplex-${version}-mac-arm64.zip`, `Simplex-${version}-mac-x64.zip`]],
		[
			`${channelName}-linux.yml`,
			[`Simplex-${version}-linux-x86_64.AppImage`, `Simplex-${version}-linux-amd64.deb`],
		],
		[
			`${channelName}-linux-arm64.yml`,
			[`Simplex-${version}-linux-arm64.AppImage`, `Simplex-${version}-linux-arm64.deb`],
		],
	])
	return { artifacts, metadata, required: [...artifacts, ...metadata], expectedUpdaterArtifacts }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const directory = resolve(process.argv[2] ?? "release-assets")
	const version = process.argv[3]
	if (!version) throw new Error("Usage: assert-release-assets.mjs <directory> <version>")
	await assertReleaseAssets(directory, version)
	process.stdout.write(`Verified Simplex desktop ${version} release assets\n`)
}
