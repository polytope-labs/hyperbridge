import { EventEmitter } from "node:events"
import type { ChildProcess, SpawnOptions } from "node:child_process"
import { fstatSync, readdirSync } from "node:fs"
import { describe, expect, it, vi } from "vitest"
import { daemonArgs, ensureDaemon, linuxDaemonStdio, spawnDaemon, type DaemonLaunch, type HealthProbe } from "../daemon"

const launch: DaemonLaunch = {
	nodePath: "/runtime/node",
	solverPath: "/app/simplex.js",
	socketPath: "/tmp/simplex.sock",
	dataDir: "/data/simplex",
}

function child(): ChildProcess & { unref: ReturnType<typeof vi.fn> } {
	const process = new EventEmitter() as ChildProcess & { unref: ReturnType<typeof vi.fn> }
	process.unref = vi.fn()
	return process
}

function sequence(states: HealthProbe[]) {
	let index = 0
	return vi.fn(async () => states[Math.min(index++, states.length - 1)])
}

describe("daemon lifecycle", () => {
	it("builds the exact bundled-runtime command line", () => {
		const args = daemonArgs(launch)
		expect(args).toEqual([
			"--enable-source-maps",
			"--disable-warning=ExperimentalWarning",
			"/app/simplex.js",
			"run",
			"--ui-socket",
			"/tmp/simplex.sock",
			"--no-open",
			"--log-format",
			"json",
			"--data-dir",
			"/data/simplex",
		])
	})

	it.each(["darwin", "win32"] as const)(
		"spawns detached with ignored stdio on %s and releases the Electron event loop",
		(platform) => {
			const spawned = child()
			const spawn = vi.fn(() => spawned)
			expect(spawnDaemon(launch, spawn as never, platform)).toBe(spawned)
			expect(spawn).toHaveBeenCalledWith(launch.nodePath, daemonArgs(launch), {
				detached: true,
				stdio: "ignore",
				windowsHide: true,
			})
			expect(spawned.unref).toHaveBeenCalledOnce()
		},
	)

	it("covers every open descriptor slot with /dev/null on Linux", () => {
		const stdio = linuxDaemonStdio(["0", "1", "2", "5", "45", "not-a-descriptor"], 9)
		expect(stdio.slice(0, 3)).toEqual(["ignore", "ignore", "ignore"])
		expect(stdio).toHaveLength(46)
		expect(new Set(stdio.slice(3))).toEqual(new Set([9]))
		expect(linuxDaemonStdio(["0", "1", "2"], 30)).toHaveLength(31)
	})

	it.runIf(process.platform === "linux")("spawns with Electron's open descriptors replaced on Linux", () => {
		const spawned = child()
		const spawn = vi.fn(() => spawned)
		spawnDaemon(launch, spawn as never, "linux")
		const [, , options] = spawn.mock.calls[0] as unknown as [string, string[], SpawnOptions]
		const stdio = options.stdio as (string | number)[]
		const highest = Math.max(...readdirSync("/proc/self/fd").map(Number))
		const devNull = stdio[3] as number
		expect(options).toMatchObject({ detached: true, windowsHide: true })
		expect(stdio.length - 1).toBeGreaterThanOrEqual(highest)
		expect(new Set(stdio.slice(3))).toEqual(new Set([devNull]))
		expect(() => fstatSync(devNull)).toThrow(/EBADF/)
		expect(spawned.unref).toHaveBeenCalledOnce()
	})

	it("attaches to a healthy daemon without spawning", async () => {
		const spawn = vi.fn()
		await expect(
			ensureDaemon({ launch, probe: sequence([{ state: "ready", mode: "operator" }]), spawn }),
		).resolves.toEqual({
			attached: true,
			mode: "operator",
		})
		expect(spawn).not.toHaveBeenCalled()
	})

	it("attaches to a stopping daemon without spawning a replacement", async () => {
		const spawn = vi.fn()
		await expect(
			ensureDaemon({ launch, probe: sequence([{ state: "stopping", mode: "operator" }]), spawn }),
		).resolves.toEqual({
			attached: true,
			mode: "operator",
		})
		expect(spawn).not.toHaveBeenCalled()
	})

	it("waits for a starting daemon without spawning a replacement", async () => {
		const spawn = vi.fn()
		await expect(
			ensureDaemon({
				launch,
				probe: sequence([
					{ state: "starting", mode: "init" },
					{ state: "starting", mode: "init" },
					{ state: "ready", mode: "operator" },
				]),
				spawn,
				delay: async () => {},
			}),
		).resolves.toEqual({ attached: true, mode: "operator" })
		expect(spawn).not.toHaveBeenCalled()
	})

	it("spawns after a stale socket and waits for health", async () => {
		const spawned = child()
		const spawn = vi.fn(() => spawned)
		const probe = sequence([
			{ state: "spawnable", reason: "stale" },
			{ state: "spawnable", reason: "stale" },
			{ state: "ready", mode: "init" },
		])
		await expect(ensureDaemon({ launch, probe, spawn, delay: async () => {} })).resolves.toEqual({
			attached: false,
			mode: "init",
		})
		expect(spawn).toHaveBeenCalledOnce()
	})

	it("never replaces a live unrecognized listener", async () => {
		const spawn = vi.fn()
		await expect(
			ensureDaemon({ launch, probe: sequence([{ state: "occupied", detail: "status 418" }]), spawn }),
		).rejects.toThrow(/socket is occupied/)
		expect(spawn).not.toHaveBeenCalled()
	})

	it("reports a child that exits before readiness", async () => {
		const spawned = child()
		const probe = sequence([
			{ state: "spawnable", reason: "absent" },
			{ state: "spawnable", reason: "absent" },
		])
		await expect(
			ensureDaemon({
				launch,
				probe,
				spawn: () => {
					queueMicrotask(() => spawned.emit("exit", 2, null))
					return spawned
				},
				delay: async () => {},
			}),
		).rejects.toThrow(/exited before its UI became ready/)
	})

	it("reports a spawn failure instead of emitting an unhandled process error", async () => {
		const spawned = child()
		await expect(
			ensureDaemon({
				launch,
				probe: sequence([{ state: "spawnable", reason: "absent" }]),
				spawn: () => {
					queueMicrotask(() => spawned.emit("error", new Error("permission denied")))
					return spawned
				},
				delay: async () => {},
			}),
		).rejects.toThrow(/could not be spawned: permission denied/)
	})

	it("times out without killing the detached child", async () => {
		const spawned = child()
		const kill = vi.fn()
		spawned.kill = kill
		await expect(
			ensureDaemon({
				launch,
				probe: sequence([{ state: "spawnable", reason: "absent" }]),
				spawn: () => spawned,
				timeoutMs: 2,
				pollIntervalMs: 1,
				delay: (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
			}),
		).rejects.toThrow(/did not answer \/health/)
		expect(kill).not.toHaveBeenCalled()
	})
})
