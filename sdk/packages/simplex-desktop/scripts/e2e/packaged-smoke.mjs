import { spawn } from "node:child_process"
import { existsSync } from "node:fs"
import { mkdtemp, readFile, readdir, realpath, rm } from "node:fs/promises"
import { request as httpRequest } from "node:http"
import { tmpdir } from "node:os"
import { basename, join, resolve } from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { fileURLToPath } from "node:url"
import { daemonArgs } from "../../src/daemon.ts"
import { socketPathFor } from "../../src/desktop-paths.ts"
import { installedAppDirectories } from "../package-size.mjs"

function socketRequest(socketPath, path, method = "GET") {
	return new Promise((resolveRequest, reject) => {
		const request = httpRequest(
			{ socketPath, path, method, headers: method === "GET" ? undefined : { "X-Simplex-UI": "1" } },
			(response) => {
				const chunks = []
				response.on("data", (chunk) => chunks.push(chunk))
				response.on("end", () =>
					resolveRequest({ status: response.statusCode ?? 0, body: Buffer.concat(chunks).toString("utf8") }),
				)
			},
		)
		request.on("error", reject)
		request.end()
	})
}

async function waitFor(check, description, timeoutMs = 120_000) {
	const deadline = Date.now() + timeoutMs
	let lastError
	while (Date.now() < deadline) {
		try {
			const result = await check()
			if (result) return result
		} catch (error) {
			lastError = error
		}
		await delay(250)
	}
	throw new Error(`Timed out waiting for ${description}${lastError ? `: ${lastError.message}` : ""}`)
}

function executableFor(appDirectory) {
	if (process.platform === "darwin") return join(appDirectory, "Contents", "MacOS", "Simplex")
	if (process.platform === "win32") return join(appDirectory, "Simplex.exe")
	const executable = join(appDirectory, "simplex")
	return existsSync(executable) ? executable : join(appDirectory, "AppRun")
}

function resourcesFor(appDirectory) {
	return process.platform === "darwin" ? join(appDirectory, "Contents", "Resources") : join(appDirectory, "resources")
}

async function stopProcess(child) {
	if (child.exitCode !== null) return
	if (!child.pid) return
	if (process.platform === "win32") {
		const killer = spawn("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" })
		await new Promise((resolveExit) => killer.once("exit", resolveExit))
	} else child.kill("SIGTERM")
	await Promise.race([new Promise((resolveExit) => child.once("exit", resolveExit)), delay(15_000)])
}

async function assertDirectSolverStartup(appDirectory) {
	const userData = await realpath(await mkdtemp(join(tmpdir(), "simplex-solver-smoke-")))
	const socketPath = socketPathFor(userData)
	const resources = resourcesFor(appDirectory)
	const launch = {
		nodePath: join(resources, "runtime", process.platform === "win32" ? "node.exe" : "node"),
		solverPath: join(resources, "simplex", "dist", "bin", "simplex.js"),
		socketPath,
		dataDir: userData,
	}
	const child = spawn(launch.nodePath, daemonArgs(launch), {
		stdio: ["ignore", "pipe", "pipe"],
		windowsHide: true,
	})
	let stderr = ""
	let spawnError
	child.once("error", (error) => {
		spawnError = error
	})
	child.stderr.on("data", (chunk) => {
		stderr += chunk.toString()
	})
	try {
		await waitFor(async () => {
			if (spawnError) throw spawnError
			if (child.exitCode !== null) throw new Error(`packaged solver exited ${child.exitCode}: ${stderr}`)
			const response = await socketRequest(socketPath, "/health")
			return response.status === 200 && JSON.parse(response.body).status === "ok"
		}, `direct packaged solver health on ${socketPath}`)
		if (stderr.trim()) throw new Error(`Packaged solver emitted stderr during startup: ${stderr.trim()}`)
		const stop = await socketRequest(socketPath, "/api/stop", "POST")
		if (stop.status !== 202)
			throw new Error(`Direct packaged solver rejected shutdown: ${stop.status} ${stop.body}`)
		await waitFor(() => child.exitCode !== null, "direct packaged solver shutdown")
	} finally {
		await stopProcess(child)
		await rm(userData, { recursive: true, force: true, maxRetries: 10, retryDelay: 250 })
	}
}

async function assertCleanSolverStartup(userData) {
	const logsDirectory = join(userData, "logs")
	const records = await waitFor(async () => {
		const names = (await readdir(logsDirectory)).filter((name) => /^simplex-[\dT-]+\.log$/.test(name)).sort()
		if (names.length !== 1) return false
		const lines = (await readFile(join(logsDirectory, names[0]), "utf8")).trim().split("\n").filter(Boolean)
		return lines.length > 0 ? lines.map((line) => JSON.parse(line)) : false
	}, "packaged solver launch log")
	const warning = records.find((record) => Number(record.level) >= 40)
	if (warning) {
		throw new Error(`Packaged solver emitted a startup warning: ${JSON.stringify(warning)}`)
	}
}

export async function smokePackagedApp(appDirectory, options = {}) {
	await assertDirectSolverStartup(appDirectory)
	const userData = await realpath(await mkdtemp(join(tmpdir(), "simplex-packaged-smoke-")))
	const socketPath = socketPathFor(userData)
	const args = [`--user-data-dir=${userData}`, "--hidden"]
	const child = spawn(options.executable ?? executableFor(appDirectory), args, {
		env: options.environment,
		stdio: ["ignore", "pipe", "pipe"],
		windowsHide: true,
	})
	let stderr = ""
	let spawnError
	child.once("error", (error) => {
		spawnError = error
	})
	child.stderr?.on("data", (chunk) => {
		stderr += chunk.toString()
	})
	try {
		const health = await waitFor(async () => {
			if (spawnError) throw spawnError
			if (child.exitCode !== null) throw new Error(`packaged app exited ${child.exitCode}: ${stderr}`)
			const response = await socketRequest(socketPath, "/health")
			if (response.status !== 200) return false
			const parsed = JSON.parse(response.body)
			return parsed.status === "ok" ? parsed : false
		}, `packaged Simplex health on ${socketPath}`)
		if (!health.pid) throw new Error("Packaged solver health did not report a PID")
		if (health.mode !== "init") throw new Error(`Fresh packaged app opened in ${health.mode} mode instead of setup`)
		const wizard = await socketRequest(socketPath, "/")
		if (wizard.status !== 200 || !wizard.body.includes('<div id="root"></div>')) {
			throw new Error(`Packaged setup wizard is unavailable: ${wizard.status}`)
		}
		await assertCleanSolverStartup(userData)
		const stop = await socketRequest(socketPath, "/api/stop", "POST")
		if (stop.status !== 202) throw new Error(`Packaged solver rejected shutdown: ${stop.status} ${stop.body}`)
		await waitFor(async () => {
			try {
				await socketRequest(socketPath, "/health")
				return false
			} catch {
				return true
			}
		}, "packaged solver shutdown")
		process.stdout.write(`Packaged smoke passed for ${basename(appDirectory)} (${health.mode})\n`)
	} finally {
		await stopProcess(child)
		await rm(userData, { recursive: true, force: true, maxRetries: 10, retryDelay: 250 })
	}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const rootIndex = process.argv.indexOf("--root")
	const releaseRoot = resolve(rootIndex === -1 ? "release" : process.argv[rootIndex + 1])
	const apps = await installedAppDirectories(releaseRoot)
	if (apps.length !== 1) throw new Error(`Expected one unpacked app under ${releaseRoot}, found ${apps.length}`)
	await smokePackagedApp(apps[0])
}
