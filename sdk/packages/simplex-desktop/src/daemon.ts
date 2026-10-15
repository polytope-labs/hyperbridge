import { spawn, type ChildProcess, type SpawnOptions, type StdioOptions } from "node:child_process"
import { closeSync, openSync, readdirSync } from "node:fs"
import { pathToFileURL } from "node:url"
import { createDiagnosticLog } from "./diagnostic-log"
import { socketRequest } from "./socket-request"

export type SimplexMode = "init" | "operator"
export type HealthProbe =
	| { state: "ready"; mode: SimplexMode; pid?: number; configEncrypted?: boolean }
	| { state: "starting"; mode: SimplexMode; pid?: number; configEncrypted?: boolean }
	| { state: "stopping"; mode: SimplexMode; pid?: number; configEncrypted?: boolean }
	| { state: "spawnable"; reason: "absent" | "stale" }
	| { state: "occupied"; detail: string }
	| { state: "unavailable"; detail: string }

export async function probeHealth(socketPath: string, timeoutMs = 1_000): Promise<HealthProbe> {
	let response: { status: number; body: string }
	try {
		response = await socketRequest(socketPath, "/health", "GET", timeoutMs)
	} catch (error) {
		const failure = error as NodeJS.ErrnoException
		if (failure.code === "ENOENT") return { state: "spawnable", reason: "absent" }
		if (failure.code === "ECONNREFUSED") return { state: "spawnable", reason: "stale" }
		return { state: "unavailable", detail: `${failure.code ?? "ERROR"}: ${failure.message}` }
	}
	try {
		const parsed = JSON.parse(response.body) as {
			status?: unknown
			mode?: unknown
			pid?: unknown
			configEncrypted?: unknown
		}
		const pidIsValid = typeof parsed.pid === "number" && Number.isSafeInteger(parsed.pid) && parsed.pid > 0
		if (
			response.status === 200 &&
			(parsed.mode === "init" || parsed.mode === "operator") &&
			(parsed.pid === undefined || pidIsValid)
		) {
			const identity: { mode: SimplexMode; pid?: number; configEncrypted?: boolean } = {
				mode: parsed.mode,
				...(pidIsValid ? { pid: parsed.pid as number } : {}),
				...(parsed.configEncrypted === true ? { configEncrypted: true } : {}),
			}
			if (parsed.status === "ok") return { state: "ready", ...identity }
			if (parsed.status === "starting") return { state: "starting", ...identity }
			if (parsed.status === "stopping") return { state: "stopping", ...identity }
		}
	} catch {
		// A listener that is not Simplex is occupied, never stale.
	}
	return { state: "occupied", detail: `Unexpected /health response (${response.status})` }
}

export interface DaemonLaunch {
	nodePath: string
	solverPath: string
	socketPath: string
	dataDir: string
	configKey?: Buffer
	diagnosticsPath?: string
}

export function daemonArgs(launch: DaemonLaunch): string[] {
	return [
		"--enable-source-maps",
		"--disable-warning=ExperimentalWarning",
		...(launch.diagnosticsPath ? ["--import", pathToFileURL(launch.diagnosticsPath).href] : []),
		launch.solverPath,
		"run",
		"--ui-socket",
		launch.socketPath,
		"--no-open",
		"--log-format",
		"json",
		"--data-dir",
		launch.dataDir,
		...(launch.configKey ? ["--config-key-stdin"] : []),
	]
}

/**
 * Stdio for the solver on Linux. Electron's main process does not mark its
 * descriptors close-on-exec, and libuv forks without closing them, so a plain
 * "ignore" would leave the detached solver holding Electron's stdout, stderr,
 * Chromium's sockets and shared memory, and any DevTools listener. libuv
 * dup2()s every numbered slot, so /dev/null in each slot up to the highest open
 * descriptor replaces all of them. macOS needs none of this: libuv spawns there
 * with POSIX_SPAWN_CLOEXEC_DEFAULT.
 */
export function linuxDaemonStdio(openDescriptors: string[], devNull: number): Array<"ignore" | number> {
	const highest = Math.max(devNull, ...openDescriptors.map(Number).filter(Number.isInteger))
	return ["ignore", "ignore", "ignore", ...Array<number>(Math.max(0, highest - 2)).fill(devNull)]
}

export function spawnDaemon(
	launch: DaemonLaunch,
	spawnImpl: typeof spawn = spawn,
	platform: NodeJS.Platform = process.platform,
): ChildProcess {
	const start = (stdio: StdioOptions) => {
		if (launch.configKey) {
			if (launch.configKey.length !== 32) throw new Error("Invalid desktop encryption key")
			stdio = Array.isArray(stdio) ? ["pipe", ...stdio.slice(1)] : ["pipe", "ignore", "ignore"]
		}
		const options: SpawnOptions = { detached: true, stdio, windowsHide: true }
		const child = spawnImpl(launch.nodePath, daemonArgs(launch), options)
		const log = createDiagnosticLog(launch.dataDir, "desktop")
		child.once("error", (error) => log("solver-spawn", error))
		child.once("exit", (code, signal) =>
			log("solver-exit", `pid=${child.pid ?? "unknown"} code=${code} signal=${signal}`),
		)
		if (launch.configKey) {
			child.stdin?.on("error", () => {})
			child.stdin?.end(launch.configKey)
		}
		child.unref()
		return child
	}
	if (platform !== "linux") return start("ignore")

	const devNull = openSync("/dev/null", "r+")
	try {
		return start(linuxDaemonStdio(readdirSync("/proc/self/fd"), devNull))
	} finally {
		closeSync(devNull)
	}
}

export async function ensureDaemon(options: {
	launch: DaemonLaunch
	probe?: (socketPath: string) => Promise<HealthProbe>
	spawn?: (launch: DaemonLaunch) => ChildProcess
	timeoutMs?: number
	pollIntervalMs?: number
	delay?: (milliseconds: number) => Promise<void>
}): Promise<{ attached: boolean; mode: SimplexMode }> {
	const probe = options.probe ?? probeHealth
	const initial = await probe(options.launch.socketPath)
	if (options.launch.configKey && "mode" in initial && !initial.configEncrypted) {
		throw new Error("The running solver must be restarted before encrypted config access")
	}
	if (initial.state === "ready") return { attached: true, mode: initial.mode }
	if (initial.state === "stopping") return { attached: true, mode: initial.mode }
	if (initial.state === "occupied") throw new Error(`The Simplex socket is occupied: ${initial.detail}`)
	if (initial.state === "unavailable") throw new Error(`The Simplex socket cannot be probed: ${initial.detail}`)

	const attached = initial.state === "starting"
	const child = attached ? undefined : (options.spawn ?? spawnDaemon)(options.launch)
	let exit: { code: number | null; signal: NodeJS.Signals | null } | undefined
	let failure: Error | undefined
	child?.once("exit", (code, signal) => {
		exit = { code, signal }
	})
	child?.once("error", (error) => {
		failure = error
	})

	const timeoutMs = options.timeoutMs ?? 120_000
	const pollIntervalMs = options.pollIntervalMs ?? 250
	const delay =
		options.delay ?? ((milliseconds: number) => new Promise((resolve) => setTimeout(resolve, milliseconds)))
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		if (failure) throw new Error(`Simplex could not be spawned: ${failure.message}`)
		if (exit) {
			throw new Error(
				`Simplex exited before its UI became ready (code ${exit.code ?? "none"}, signal ${exit.signal ?? "none"})`,
			)
		}
		const health = await probe(options.launch.socketPath)
		if (options.launch.configKey && "mode" in health && !health.configEncrypted) {
			throw new Error("The solver does not support encrypted config storage")
		}
		if (health.state === "ready") return { attached, mode: health.mode }
		if (health.state === "stopping") return { attached, mode: health.mode }
		if (attached && health.state === "spawnable") {
			throw new Error(`Simplex stopped before its UI became ready (${health.reason})`)
		}
		if (health.state === "occupied")
			throw new Error(`The Simplex socket was taken by an unexpected listener: ${health.detail}`)
		await delay(pollIntervalMs)
	}
	throw new Error(`Simplex did not answer /health within ${Math.ceil(timeoutMs / 1_000)} seconds`)
}
