import { spawn, type ChildProcess, type SpawnOptions, type StdioOptions } from "node:child_process"
import { closeSync, openSync, readdirSync } from "node:fs"
import { request as httpRequest } from "node:http"

export type SimplexMode = "init" | "operator"
export type HealthProbe =
	| { state: "ready"; mode: SimplexMode }
	| { state: "starting"; mode: SimplexMode }
	| { state: "stopping"; mode: SimplexMode }
	| { state: "spawnable"; reason: "absent" | "stale" }
	| { state: "occupied"; detail: string }
	| { state: "unavailable"; detail: string }

export function probeHealth(socketPath: string, timeoutMs = 1_000): Promise<HealthProbe> {
	return new Promise((resolve) => {
		const request = httpRequest({ socketPath, path: "/health", method: "GET" }, (response) => {
			let body = ""
			response.setEncoding("utf8")
			response.on("data", (chunk: string) => {
				body += chunk
			})
			response.on("end", () => {
				try {
					const parsed = JSON.parse(body) as { status?: unknown; mode?: unknown }
					if (response.statusCode === 200 && (parsed.mode === "init" || parsed.mode === "operator")) {
						if (parsed.status === "ok") resolve({ state: "ready", mode: parsed.mode })
						else if (parsed.status === "starting") resolve({ state: "starting", mode: parsed.mode })
						else if (parsed.status === "stopping") resolve({ state: "stopping", mode: parsed.mode })
						else throw new Error("unrecognized health status")
						return
					}
				} catch {
					// A listener that is not Simplex is occupied, never stale.
				}
				resolve({
					state: "occupied",
					detail: `Unexpected /health response (${response.statusCode ?? "no status"})`,
				})
			})
		})
		request.setTimeout(timeoutMs, () =>
			request.destroy(Object.assign(new Error("health probe timed out"), { code: "ETIMEDOUT" })),
		)
		request.on("error", (error: NodeJS.ErrnoException) => {
			if (error.code === "ENOENT") resolve({ state: "spawnable", reason: "absent" })
			else if (error.code === "ECONNREFUSED") resolve({ state: "spawnable", reason: "stale" })
			else resolve({ state: "unavailable", detail: `${error.code ?? "ERROR"}: ${error.message}` })
		})
		request.end()
	})
}

export interface DaemonLaunch {
	nodePath: string
	solverPath: string
	socketPath: string
	dataDir: string
}

export function daemonArgs(launch: DaemonLaunch): string[] {
	return [
		"--enable-source-maps",
		"--disable-warning=ExperimentalWarning",
		launch.solverPath,
		"run",
		"--ui-socket",
		launch.socketPath,
		"--no-open",
		"--log-format",
		"json",
		"--data-dir",
		launch.dataDir,
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
		const options: SpawnOptions = { detached: true, stdio, windowsHide: true }
		const child = spawnImpl(launch.nodePath, daemonArgs(launch), options)
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
