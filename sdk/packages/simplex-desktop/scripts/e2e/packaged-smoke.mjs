import { spawn } from "node:child_process"
import { existsSync } from "node:fs"
import { mkdtemp, readFile, readdir, realpath, rm, stat } from "node:fs/promises"
import { request as httpRequest } from "node:http"
import { tmpdir } from "node:os"
import { basename, join, resolve } from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { fileURLToPath } from "node:url"
import { daemonArgs } from "../../src/daemon.ts"
import { socketPathFor } from "../../src/desktop-paths.ts"
import { installedAppDirectories } from "../package-size.mjs"

function socketRequest(socketPath, path, method = "GET", body = undefined) {
	return new Promise((resolveRequest, reject) => {
		const encodedBody = body === undefined ? undefined : Buffer.from(JSON.stringify(body))
		const headers =
			method === "GET"
				? undefined
				: {
						"X-Simplex-UI": "1",
						...(encodedBody
							? { "Content-Type": "application/json", "Content-Length": String(encodedBody.byteLength) }
							: {}),
					}
		const request = httpRequest(
			{ socketPath, path, method, headers },
			(response) => {
				const chunks = []
				response.on("data", (chunk) => chunks.push(chunk))
				response.on("end", () =>
					resolveRequest({ status: response.statusCode ?? 0, body: Buffer.concat(chunks).toString("utf8") }),
				)
			},
		)
		request.on("error", reject)
		request.end(encodedBody)
	})
}

const SETUP_READY_LOG = "No config found, starting the setup wizard"
const RETRYABLE_SOCKET_ERRORS = new Set(["ENOENT", "ECONNREFUSED", "ECONNRESET"])

function retryableSocketError(error) {
	return RETRYABLE_SOCKET_ERRORS.has(error?.code)
}

export async function waitFor(check, description, timeoutMs = 120_000, pause = delay) {
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const result = await check()
		if (result) return result
		await pause(250)
	}
	throw new Error(`Timed out waiting for ${description}`)
}

function waitForProcessExit(child, timeoutMs) {
	if (child.exitCode !== null) return Promise.resolve(true)
	return new Promise((resolveExit) => {
		const onExit = () => {
			clearTimeout(timer)
			resolveExit(true)
		}
		const timer = setTimeout(() => {
			child.off("exit", onExit)
			resolveExit(false)
		}, timeoutMs)
		child.once("exit", onExit)
	})
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

export async function stopProcess(
	child,
	{ platform = process.platform, gracePeriodMs = 15_000, forcePeriodMs = 5_000 } = {},
) {
	if (child.exitCode !== null) return
	if (!child.pid) return
	if (platform === "win32") {
		const killer = spawn("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" })
		await new Promise((resolveExit) => killer.once("exit", resolveExit))
		if (await waitForProcessExit(child, forcePeriodMs)) return
		throw new Error(`Packaged app process ${child.pid} remained alive after taskkill`)
	}

	child.kill("SIGTERM")
	if (await waitForProcessExit(child, gracePeriodMs)) return

	child.kill("SIGKILL")
	if (await waitForProcessExit(child, forcePeriodMs)) return
	throw new Error(`Packaged app process ${child.pid} remained alive after SIGKILL`)
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
			try {
				const response = await socketRequest(socketPath, "/health")
				return response.status === 200 && JSON.parse(response.body).status === "ok"
			} catch (error) {
				if (retryableSocketError(error)) return false
				throw error
			}
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
		try {
			const names = (await readdir(logsDirectory)).filter((name) => /^simplex-[\dT-]+\.log$/.test(name)).sort()
			if (names.length !== 1) return false
			const lines = (await readFile(join(logsDirectory, names[0]), "utf8")).trim().split("\n").filter(Boolean)
			const parsed = lines.map((line) => JSON.parse(line))
			return parsed.some((record) => record.msg === SETUP_READY_LOG) ? parsed : false
		} catch (error) {
			if (error?.code === "ENOENT" || error instanceof SyntaxError) return false
			throw error
		}
	}, "packaged solver startup-complete log record")
	const warning = records.find((record) => Number(record.level) >= 40)
	if (warning) {
		throw new Error(`Packaged solver emitted a startup warning: ${JSON.stringify(warning)}`)
	}
}

const PACKAGED_SETUP_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"

async function assertPackagedOnboarding(socketPath, userData) {
	const configPath = join(userData, "filler-config.toml")
	const config = {
		simplex: {
			signer: { type: "privateKey", key: PACKAGED_SETUP_KEY },
			maxConcurrentOrders: 1,
			substratePrivateKey: "bottom drive obey lake curtain smoke basket hold race lonely fit walk",
			hyperbridgeWsUrl: "ws://127.0.0.1:9",
		},
		pairs: [
			{
				token0: "USDC",
				token1: "USDC",
				maxOrderSize: "100000",
				askPriceCurve: [
					{ amount: "100", price: "0.99" },
					{ amount: "100000", price: "0.999" },
				],
			},
		],
		chains: [{ rpcUrls: ["http://127.0.0.1:9"], bundlerUrl: "http://127.0.0.1:9" }],
		orderbook: { url: "https://orderbook.example/graphql" },
	}
	const response = await socketRequest(socketPath, "/api/setup/save-and-start", "POST", { config })
	if (response.status !== 202) {
		throw new Error(`Packaged setup rejected config: ${response.status} ${response.body}`)
	}
	const result = JSON.parse(response.body)
	if (result.configPath !== configPath) {
		throw new Error(`Packaged setup wrote ${result.configPath}, expected ${configPath}`)
	}
	await waitFor(() => existsSync(configPath), "packaged first-run config")
	const written = await readFile(configPath, "utf8")
	if (!written.includes("WARNING: contains secrets") || !written.includes(PACKAGED_SETUP_KEY)) {
		throw new Error("Packaged first-run config is incomplete")
	}
	if (process.platform !== "win32" && ((await stat(configPath)).mode & 0o777) !== 0o600) {
		throw new Error("Packaged first-run config is not mode 0600")
	}
	await waitFor(async () => {
		const status = await socketRequest(socketPath, "/api/setup/start-status")
		if (status.status !== 200) return false
		const state = JSON.parse(status.body).state
		if (state === "running") throw new Error("Offline packaged setup unexpectedly entered operator mode")
		return state === "failed"
	}, "offline packaged setup attempt to fail closed")
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
			try {
				const response = await socketRequest(socketPath, "/health")
				if (response.status !== 200) return false
				const parsed = JSON.parse(response.body)
				return parsed.status === "ok" ? parsed : false
			} catch (error) {
				if (retryableSocketError(error)) return false
				throw error
			}
		}, `packaged Simplex health on ${socketPath}`)
		if (!health.pid) throw new Error("Packaged solver health did not report a PID")
		if (health.mode !== "init") throw new Error(`Fresh packaged app opened in ${health.mode} mode instead of setup`)
		const wizard = await socketRequest(socketPath, "/")
		if (wizard.status !== 200 || !wizard.body.includes('<div id="root"></div>')) {
			throw new Error(`Packaged setup wizard is unavailable: ${wizard.status}`)
		}
		await assertCleanSolverStartup(userData)
		await assertPackagedOnboarding(socketPath, userData)
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
		process.stdout.write(`Packaged smoke passed setup, config write, and fail-closed boot for ${basename(appDirectory)}\n`)
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
