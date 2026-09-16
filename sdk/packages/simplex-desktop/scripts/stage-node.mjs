import { spawnSync } from "node:child_process"
import { createHash } from "node:crypto"
import { createReadStream, createWriteStream } from "node:fs"
import { chmod, copyFile, mkdir, mkdtemp, readFile, rename, rm, stat } from "node:fs/promises"
import { tmpdir } from "node:os"
import { basename, dirname, join, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { pipeline } from "node:stream/promises"
import * as openpgp from "openpgp"
import * as tar from "tar"

export const NODE_VERSION = "24.19.0"
export const NODE_BASE_URL = `https://nodejs.org/dist/v${NODE_VERSION}`
export const NODE_RELEASE_KEY_FINGERPRINT = "5BE8A3F6C8A5C01D106C0AD820B1A390B168D356"
export const TARGETS = ["darwin-arm64", "darwin-x64", "darwin-universal", "linux-arm64", "linux-x64", "win32-x64"]

const scriptDir = dirname(fileURLToPath(import.meta.url))
const packageRoot = resolve(scriptDir, "..")

export function assetFor(target) {
	if (target === "win32-x64") return "win-x64/node.exe"
	if (target === "darwin-universal") throw new Error("darwin-universal is assembled from the two Darwin slices")
	if (!TARGETS.includes(target)) throw new Error(`Unsupported Node runtime target: ${target}`)
	const [platform, arch] = target.split("-")
	return `node-v${NODE_VERSION}-${platform}-${arch}.tar.gz`
}

export function checksumFromManifest(manifest, asset) {
	const matches = manifest
		.split(/\r?\n/)
		.map((line) => line.match(/^([0-9a-f]{64})  (.+)$/))
		.filter((match) => match?.[2] === asset)
	if (matches.length !== 1)
		throw new Error(`Signed checksum manifest contains ${matches.length} entries for ${asset}`)
	return matches[0][1]
}

export async function verifyManifestSignature(signedManifest, armoredKey, expectedFingerprint) {
	const message = await openpgp.readCleartextMessage({ cleartextMessage: signedManifest })
	const verificationKeys = await openpgp.readKey({ armoredKey })
	if (expectedFingerprint && verificationKeys.getFingerprint().toUpperCase() !== expectedFingerprint) {
		throw new Error(`Node release key fingerprint does not match the pinned signer: ${expectedFingerprint}`)
	}
	const verification = await openpgp.verify({ message, verificationKeys })
	if (verification.signatures.length !== 1) {
		throw new Error(`Expected one Node release signature, got ${verification.signatures.length}`)
	}
	await verification.signatures[0].verified
	return message.getText()
}

async function download(url, destination) {
	const response = await fetch(url, { redirect: "follow" })
	if (!response.ok || !response.body) throw new Error(`Download failed (${response.status}) for ${url}`)
	await pipeline(response.body, createWriteStream(destination, { flags: "wx" }))
}

async function sha256(path) {
	const hash = createHash("sha256")
	await pipeline(createReadStream(path), hash)
	return hash.digest("hex")
}

async function verifiedDownload(asset, directory, manifest) {
	const destination = join(directory, basename(asset))
	await download(`${NODE_BASE_URL}/${asset}`, destination)
	const expected = checksumFromManifest(manifest, asset)
	const actual = await sha256(destination)
	if (actual !== expected) throw new Error(`SHA-256 mismatch for ${asset}: expected ${expected}, got ${actual}`)
	return destination
}

export async function extractNode(archive, destination) {
	const root = basename(archive, ".tar.gz")
	let extracted = false
	const extractionDirectory = await mkdtemp(join(dirname(destination), ".simplex-node-extract-"))
	try {
		await tar.x({
			file: archive,
			cwd: extractionDirectory,
			strip: 2,
			filter(path, entry) {
				const selected = path === `${root}/bin/node` && entry.type === "File"
				if (selected) extracted = true
				return selected
			},
		})
		if (!extracted) throw new Error(`The verified archive ${basename(archive)} did not contain bin/node`)
		await rename(join(extractionDirectory, "node"), destination)
	} finally {
		await rm(extractionDirectory, { recursive: true, force: true })
	}
}

async function stageSlice(target, workingDirectory, manifest) {
	const asset = assetFor(target)
	const downloaded = await verifiedDownload(asset, workingDirectory, manifest)
	const executable = join(workingDirectory, target === "win32-x64" ? "node.exe" : `node-${target}`)
	if (target === "win32-x64") await rename(downloaded, executable)
	else await extractNode(downloaded, executable)
	if (target !== "win32-x64") await chmod(executable, 0o755)
	return executable
}

export async function atomicInstall(source, destination, executable) {
	await mkdir(dirname(destination), { recursive: true })
	// The verified download lives under the OS temp directory, which can be a
	// different volume from the workspace on Windows runners. Copy into a unique
	// sibling first, then make the only visible transition with a same-volume
	// rename so readers never observe a partial runtime.
	const temporaryDirectory = await mkdtemp(join(dirname(destination), `.${basename(destination)}.new-`))
	const temporary = join(temporaryDirectory, basename(destination))
	try {
		await copyFile(source, temporary)
		if (executable) await chmod(temporary, 0o755)
		// rename replaces an existing file without exposing a partially written runtime.
		await rename(temporary, destination)
	} finally {
		await rm(temporaryDirectory, { recursive: true, force: true })
	}
}

export function mergeUniversal(arm64, x64, destination, spawnImpl = spawnSync) {
	const result = spawnImpl("/usr/bin/lipo", ["-create", arm64, x64, "-output", destination], { encoding: "utf8" })
	if (result.status !== 0) throw new Error(`lipo failed: ${result.stderr || result.stdout}`)
}

export function defaultTarget(platform = process.platform, arch = process.arch) {
	const target = `${platform}-${arch}`
	if (!TARGETS.includes(target)) throw new Error(`No bundled Node runtime target for ${target}`)
	return target
}

function parseTarget(argv) {
	const index = argv.indexOf("--target")
	return index === -1 ? defaultTarget() : argv[index + 1]
}

export async function stageNode(target, outputRoot = resolve(packageRoot, "resources/node")) {
	if (!TARGETS.includes(target)) throw new Error(`Unsupported Node runtime target: ${target}`)
	const workingDirectory = await mkdtemp(join(tmpdir(), "simplex-node-stage-"))
	try {
		const [signedManifest, armoredKey] = await Promise.all([
			(async () => {
				const path = join(workingDirectory, "SHASUMS256.txt.asc")
				await download(`${NODE_BASE_URL}/SHASUMS256.txt.asc`, path)
				return readFile(path, "utf8")
			})(),
			readFile(resolve(scriptDir, "node-release-key.asc"), "utf8"),
		])
		const manifest = await verifyManifestSignature(signedManifest, armoredKey, NODE_RELEASE_KEY_FINGERPRINT)
		const destinationDir = resolve(outputRoot, target)
		const destination = join(destinationDir, target === "win32-x64" ? "node.exe" : "node")

		if (target === "darwin-universal") {
			if (process.platform !== "darwin") throw new Error("darwin-universal must be staged on macOS with lipo")
			const [arm64, x64] = await Promise.all([
				stageSlice("darwin-arm64", workingDirectory, manifest),
				stageSlice("darwin-x64", workingDirectory, manifest),
			])
			const merged = join(workingDirectory, "node-universal")
			mergeUniversal(arm64, x64, merged)
			await atomicInstall(merged, destination, true)
		} else {
			const staged = await stageSlice(target, workingDirectory, manifest)
			await atomicInstall(staged, destination, target !== "win32-x64")
		}

		const installed = await stat(destination)
		if (!installed.isFile() || installed.size === 0)
			throw new Error(`Staged runtime is not a non-empty file: ${destination}`)
		return destination
	} finally {
		await rm(workingDirectory, { recursive: true, force: true })
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
	const target = parseTarget(process.argv.slice(2))
	stageNode(target)
		.then((path) => process.stdout.write(`Staged Node v${NODE_VERSION} for ${target} at ${path}\n`))
		.catch((error) => {
			process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
			process.exitCode = 1
		})
}
