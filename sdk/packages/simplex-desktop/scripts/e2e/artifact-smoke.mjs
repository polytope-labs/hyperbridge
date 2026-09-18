import { spawn } from "node:child_process"
import { chmod, mkdir, mkdtemp, readdir, rm } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { smokePackagedApp } from "./packaged-smoke.mjs"

function run(command, args, options = {}) {
	return new Promise((resolveRun, reject) => {
		const child = spawn(command, args, { ...options, stdio: ["ignore", "pipe", "pipe"] })
		let output = ""
		child.stdout.on("data", (chunk) => {
			output += chunk.toString()
		})
		child.stderr.on("data", (chunk) => {
			output += chunk.toString()
		})
		child.once("error", reject)
		child.once("exit", (code, signal) => {
			if (code === 0) resolveRun(output)
			else reject(new Error(`${command} exited ${code ?? signal}: ${output}`))
		})
	})
}

function exactlyOne(names, predicate, description) {
	const matches = names.filter(predicate)
	if (matches.length !== 1) throw new Error(`Expected one ${description}, found ${matches.join(", ") || "none"}`)
	return matches[0]
}

export function artifactNamesForPlatform(names, platform = process.platform) {
	if (platform === "darwin") {
		return [
			exactlyOne(names, (name) => name.endsWith(".dmg"), "macOS DMG"),
			exactlyOne(names, (name) => name.endsWith(".zip"), "macOS updater ZIP"),
		]
	}
	if (platform === "win32") {
		return [exactlyOne(names, (name) => name.endsWith(".exe"), "Windows NSIS installer")]
	}
	if (platform === "linux") {
		return [
			exactlyOne(names, (name) => name.endsWith(".AppImage"), "Linux AppImage"),
			exactlyOne(names, (name) => name.endsWith(".deb"), "Linux deb"),
		]
	}
	throw new Error(`No installer smoke test for ${platform}`)
}

export function isUnavailableAppImageFuse(error) {
	return /fusermount\d*: mount failed: Operation not permitted/.test(
		error instanceof Error ? error.message : String(error),
	)
}

async function smokeMacArtifact(artifact, temporary) {
	if (artifact.endsWith(".zip")) {
		const extracted = join(temporary, "zip")
		await mkdir(extracted)
		await run("ditto", ["-x", "-k", artifact, extracted])
		await smokePackagedApp(join(extracted, "Simplex.app"))
		return
	}

	const mount = join(temporary, "dmg")
	await mkdir(mount)
	await run("hdiutil", ["attach", "-nobrowse", "-readonly", "-mountpoint", mount, artifact])
	try {
		await smokePackagedApp(join(mount, "Simplex.app"))
	} finally {
		await run("hdiutil", ["detach", mount])
	}
}

async function smokeLinuxArtifact(artifact, temporary) {
	if (artifact.endsWith(".deb")) {
		if (process.env.CI === "true") {
			const packageName = (await run("dpkg-deb", ["--field", artifact, "Package"])).trim()
			await run("sudo", ["-n", "apt-get", "install", "--yes", "--no-install-recommends", artifact])
			try {
				await smokePackagedApp(join("/opt", "Simplex"))
			} finally {
				await run("sudo", ["-n", "dpkg", "--remove", packageName])
			}
			return
		}
		const extracted = join(temporary, "deb")
		await mkdir(extracted)
		await run("dpkg-deb", ["--extract", artifact, extracted])
		await smokePackagedApp(join(extracted, "opt", "Simplex"))
		return
	}

	await chmod(artifact, 0o755)
	const extracted = join(temporary, "appimage")
	await mkdir(extracted)
	await run(artifact, ["--appimage-extract"], { cwd: extracted })
	const appDirectory = join(extracted, "squashfs-root")
	try {
		await smokePackagedApp(appDirectory, { executable: artifact })
	} catch (error) {
		if (!isUnavailableAppImageFuse(error)) throw error
		process.stderr.write("Runner FUSE is unavailable; retrying the AppImage through its self-extract runtime\n")
		await smokePackagedApp(appDirectory, {
			executable: artifact,
			environment: { ...process.env, APPIMAGE_EXTRACT_AND_RUN: "1" },
		})
	}
}

async function smokeWindowsArtifact(artifact, temporary) {
	const installed = join(temporary, "installed")
	await run(artifact, ["/S", `/D=${installed}`], { windowsHide: true })
	await smokePackagedApp(installed)
}

export async function smokeReleaseArtifacts(releaseRoot) {
	const directory = resolve(releaseRoot)
	const names = await readdir(directory)
	const artifacts = artifactNamesForPlatform(names).map((name) => join(directory, name))
	for (const artifact of artifacts) {
		const temporary = await mkdtemp(join(tmpdir(), "simplex-artifact-smoke-"))
		try {
			if (process.platform === "darwin") await smokeMacArtifact(artifact, temporary)
			else if (process.platform === "win32") await smokeWindowsArtifact(artifact, temporary)
			else await smokeLinuxArtifact(artifact, temporary)
			process.stdout.write(`Artifact smoke passed for ${artifact}\n`)
		} finally {
			await rm(temporary, { recursive: true, force: true, maxRetries: 10, retryDelay: 250 })
		}
	}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const rootIndex = process.argv.indexOf("--root")
	await smokeReleaseArtifacts(rootIndex === -1 ? "release" : process.argv[rootIndex + 1])
}
