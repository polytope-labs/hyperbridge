import { EventEmitter } from "node:events"
import { describe, expect, it, vi } from "vitest"
import { UpdateCoordinator, waitForSolverExit, type UpdaterAdapter } from "../update-coordinator"
import type { UpdatePreferences, UpdateStore } from "../update-store"

class FakeUpdater extends EventEmitter implements UpdaterAdapter {
	autoDownload = false
	autoInstallOnAppQuit = true
	allowPrerelease = false
	allowDowngrade = true
	private selectedChannel: string | null = null
	get channel(): string | null {
		return this.selectedChannel
	}
	set channel(value: string | null) {
		this.selectedChannel = value
		// Match electron-updater: selecting a channel implicitly enables downgrades.
		this.allowDowngrade = true
	}
	checkForUpdates = vi.fn<UpdaterAdapter["checkForUpdates"]>(async () => undefined)
	quitAndInstall = vi.fn()
}

class MemoryUpdateStore implements UpdateStore {
	constructor(public value: UpdatePreferences = { channel: "stable" }) {}
	read(): UpdatePreferences {
		return structuredClone(this.value)
	}
	write(value: UpdatePreferences): void {
		this.value = structuredClone(value)
	}
}

function idleSolver() {
	return {
		state: "running" as const,
		pid: 42,
		version: "0.16.2",
		work: {
			queuedEvaluations: 0,
			evaluating: 0,
			queuedFills: 0,
			activeFills: 0,
			retractions: 0,
			rebalancing: 0,
		},
	}
}

function coordinator(overrides: Partial<ConstructorParameters<typeof UpdateCoordinator>[0]> = {}) {
	const updater = new FakeUpdater()
	const store = new MemoryUpdateStore()
	const options: ConstructorParameters<typeof UpdateCoordinator>[0] = {
		updater,
		packaged: true,
		appVersion: "0.16.2",
		store,
		probeSolver: vi.fn(async () => idleSolver()),
		canRestartSolver: () => true,
		requestSolverStop: vi.fn(async () => undefined),
		restartSolver: vi.fn(async () => undefined),
		waitForExit: vi.fn(async () => undefined),
		onChange: vi.fn(),
		notify: vi.fn(),
		...overrides,
	}
	return { updater, store, options, instance: new UpdateCoordinator(options) }
}

function finishDownload(updater: FakeUpdater, version: string): void {
	updater.emit("update-available", { version })
	updater.emit("update-downloaded", { version })
}

describe("desktop update coordinator", () => {
	it("configures manual installation and stable checks in packaged builds", async () => {
		const test = coordinator()
		test.instance.start()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalled())
		expect(test.updater.autoDownload).toBe(true)
		expect(test.updater.autoInstallOnAppQuit).toBe(false)
		expect(test.updater.allowDowngrade).toBe(false)
		expect(test.updater.allowPrerelease).toBe(false)
		expect(test.updater.channel).toBe("latest")
		test.instance.dispose()
	})

	it("downloads an update and leaves installing it to the operator", async () => {
		const test = coordinator()
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		expect(test.instance.status).toMatchObject({ state: "ready", targetVersion: "0.17.0" })
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(test.options.probeSolver).not.toHaveBeenCalled()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		expect(test.store.value.receipt).toBeUndefined()
		test.instance.dispose()
	})

	it("does not check again while a downloaded update waits to be installed", async () => {
		const test = coordinator()
		test.instance.start()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalledOnce())
		finishDownload(test.updater, "0.17.0")
		await test.instance.checkNow()
		expect(test.updater.checkForUpdates).toHaveBeenCalledOnce()
		expect(test.instance.status).toMatchObject({ state: "ready", targetVersion: "0.17.0" })
		test.instance.dispose()
	})

	it("refuses to install before an update has been downloaded", async () => {
		const test = coordinator()
		test.instance.start()
		await expect(test.instance.install()).rejects.toThrow("No downloaded update")
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("stops, proves process exit, and only then invokes the installer", async () => {
		const order: string[] = []
		const test = coordinator({
			requestSolverStop: vi.fn(async () => {
				order.push("stop")
			}),
			waitForExit: vi.fn(async () => {
				order.push("exit")
			}),
		})
		test.updater.quitAndInstall.mockImplementation(() => order.push("install"))
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await test.instance.install()
		expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true)
		expect(order).toEqual(["stop", "exit", "install"])
		expect(test.store.value.receipt).toEqual({
			fromVersion: "0.16.2",
			targetVersion: "0.17.0",
			installAttemptedAt: expect.any(Number),
		})
		expect(test.instance.status).toMatchObject({ state: "installing", targetVersion: "0.17.0" })
		test.instance.dispose()
	})

	it("installs with fills still in flight, which the solver drains as it stops", async () => {
		const test = coordinator({
			probeSolver: vi.fn(async () => ({ ...idleSolver(), work: { ...idleSolver().work, activeFills: 1 } })),
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await test.instance.install()
		expect(test.options.requestSolverStop).toHaveBeenCalledOnce()
		expect(test.updater.quitAndInstall).toHaveBeenCalledOnce()
		test.instance.dispose()
	})

	it("runs one install when it is asked for twice", async () => {
		const test = coordinator()
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await Promise.all([test.instance.install(), test.instance.install()])
		expect(test.options.requestSolverStop).toHaveBeenCalledOnce()
		expect(test.updater.quitAndInstall).toHaveBeenCalledOnce()
		test.instance.dispose()
	})

	it("waits for a slow solver without a deadline, and lets nothing else touch the updater meanwhile", async () => {
		let exited!: () => void
		const test = coordinator({
			waitForExit: vi.fn(
				() =>
					new Promise<void>((resolve) => {
						exited = resolve
					}),
			),
		})
		test.instance.start()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalledOnce())
		finishDownload(test.updater, "0.17.0")
		const install = test.instance.install()
		await vi.waitFor(() => expect(test.options.waitForExit).toHaveBeenCalledWith(42))
		expect(test.instance.status).toMatchObject({ state: "stopping-solver", targetVersion: "0.17.0" })

		await test.instance.checkNow()
		test.instance.setChannel("beta")
		expect(test.updater.checkForUpdates).toHaveBeenCalledOnce()
		expect(test.store.value.channel).toBe("stable")
		expect(test.instance.status.state).toBe("stopping-solver")
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()

		exited()
		await install
		expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true)
		// The installer is running: the updater stays closed until the app quits.
		await test.instance.checkNow()
		test.instance.setChannel("beta")
		expect(test.updater.checkForUpdates).toHaveBeenCalledOnce()
		expect(test.store.value.channel).toBe("stable")
		test.instance.dispose()
	})

	it("keeps the update ready when the solver refuses the stop", async () => {
		const test = coordinator({
			requestSolverStop: vi.fn(async () => {
				throw new Error("Simplex could not stop: 503")
			}),
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow("could not stop")
		expect(test.instance.status).toMatchObject({ state: "ready", targetVersion: "0.17.0" })
		expect(test.options.waitForExit).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("goes on with the install when the stop request fails but the solver took it", async () => {
		const test = coordinator({
			requestSolverStop: vi.fn(async () => {
				throw new Error("Simplex could not stop: timed out")
			}),
		})
		vi.mocked(test.options.probeSolver)
			.mockResolvedValueOnce(idleSolver())
			.mockResolvedValue({ state: "stopping", pid: 42 })
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await test.instance.install()
		expect(test.options.waitForExit).toHaveBeenCalledWith(42)
		expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true)
		test.instance.dispose()
	})

	it("does not offer the download again when the updater failed before the stop was refused", async () => {
		const test = coordinator()
		vi.mocked(test.options.requestSolverStop).mockImplementation(async () => {
			test.updater.emit("error", new Error("update file is gone"))
			throw new Error("Simplex could not stop: 503")
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow("could not stop")
		expect(test.instance.status).toMatchObject({ state: "error", detail: "update file is gone" })
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("restarts the solver instead of installing when the updater fails while the solver stops", async () => {
		let exited!: () => void
		const test = coordinator({
			waitForExit: vi.fn(
				() =>
					new Promise<void>((resolve) => {
						exited = resolve
					}),
			),
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		const install = test.instance.install()
		await vi.waitFor(() => expect(test.options.waitForExit).toHaveBeenCalled())
		test.updater.emit("error", new Error("update file is gone"))
		exited()
		await expect(install).rejects.toThrow("update file is gone")
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		expect(test.options.restartSolver).toHaveBeenCalledOnce()
		expect(test.store.value.receipt).toBeUndefined()
		expect(test.instance.status.state).toBe("error")
		test.instance.dispose()
	})

	it("keeps the channel the updater follows when the new one cannot be saved", () => {
		const test = coordinator()
		test.instance.start()
		vi.spyOn(test.store, "write").mockImplementation(() => {
			throw new Error("disk unavailable")
		})
		test.instance.setChannel("beta")
		expect(test.updater.channel).toBe("latest")
		expect(test.instance.status).toMatchObject({ state: "error", channel: "stable" })
		test.instance.dispose()
	})

	it("enables beta without allowing a downgrade", () => {
		const test = coordinator()
		test.instance.setChannel("beta")
		expect(test.updater.channel).toBe("beta")
		expect(test.updater.allowPrerelease).toBe(true)
		expect(test.updater.allowDowngrade).toBe(false)
		expect(test.store.value.channel).toBe("beta")
	})

	it("discards a downloaded update when the operator changes channels", async () => {
		const store = new MemoryUpdateStore({ channel: "beta" })
		const test = coordinator({ store })
		test.instance.start()
		finishDownload(test.updater, "0.17.0-beta.1")
		expect(test.instance.status.state).toBe("ready")
		test.instance.setChannel("stable")
		expect(test.instance.status.state).not.toBe("ready")
		expect(test.updater.allowDowngrade).toBe(false)
		await expect(test.instance.install()).rejects.toThrow("No downloaded update")
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("rejects a late download from the channel the operator left", async () => {
		const store = new MemoryUpdateStore({ channel: "beta" })
		const test = coordinator({ store })
		let finishOldDownload!: () => void
		const oldDownload = new Promise<void>((resolve) => {
			finishOldDownload = resolve
		})
		test.updater.checkForUpdates.mockResolvedValueOnce({ downloadPromise: oldDownload })
		test.instance.start()
		await Promise.resolve()
		test.instance.setChannel("stable")
		// The old beta check may emit only after the channel preference changes.
		test.updater.emit("update-available", { version: "0.17.0-beta.1" })
		test.updater.emit("update-downloaded", { version: "0.17.0-beta.1" })
		finishOldDownload()

		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(store.value.channel).toBe("stable")
		expect(test.instance.status.state).not.toBe("ready")
		await expect(test.instance.install()).rejects.toThrow("No downloaded update")
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("cancels an install when the channel changes during its solver probe", async () => {
		let resolveProbe!: (status: ReturnType<typeof idleSolver>) => void
		const probeSolver = vi.fn(
			() =>
				new Promise<ReturnType<typeof idleSolver>>((resolve) => {
					resolveProbe = resolve
				}),
		)
		const test = coordinator({ probeSolver })
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		const install = test.instance.install()
		await vi.waitFor(() => expect(probeSolver).toHaveBeenCalled())

		test.instance.setChannel("beta")
		resolveProbe(idleSolver())
		await expect(install).rejects.toThrow("discarded")

		expect(test.store.value.receipt).toBeUndefined()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("clears an attempted receipt only after app and solver versions match", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: { fromVersion: "0.16.2", targetVersion: "0.17.0", installAttemptedAt: 2 },
		})
		const test = coordinator({
			appVersion: "0.17.0",
			store,
			probeSolver: vi.fn(async () => ({ ...idleSolver(), version: "0.17.0" })),
		})
		test.instance.start()
		await vi.waitFor(() => expect(store.value.receipt).toBeUndefined())
		expect(test.options.notify).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("accepts a healthy matching setup solver after an update", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: { fromVersion: "0.16.2", targetVersion: "0.17.0", installAttemptedAt: 2 },
		})
		const test = coordinator({
			appVersion: "0.17.0",
			store,
			probeSolver: vi.fn(async () => ({ state: "setup" as const, pid: 42, version: "0.17.0" })),
		})
		test.instance.start()
		await vi.waitFor(() => expect(store.value.receipt).toBeUndefined())
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalled())
		expect(test.options.notify).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("does not stop a solver that it could not restart", async () => {
		let hasRestartKey = false
		const test = coordinator({ canRestartSolver: () => hasRestartKey })
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow("cannot restart the solver")
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		expect(test.instance.status.state).toBe("ready")

		hasRestartKey = true
		await test.instance.install()
		expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true)
		test.instance.dispose()
	})

	it("restarts the solver and rejects when electron-updater emits an installer error at once", async () => {
		const test = coordinator()
		test.updater.quitAndInstall.mockImplementation(() => {
			test.updater.emit("error", new Error("installer unavailable"))
		})
		test.instance.start()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalledOnce())
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow(
			"Update 0.17.0 could not be installed: installer unavailable",
		)
		expect(test.store.value.receipt).toBeUndefined()
		expect(test.options.restartSolver).toHaveBeenCalledOnce()
		// The rejection is the report, and the failed download is not offered again by itself.
		expect(test.options.notify).not.toHaveBeenCalled()
		expect(test.instance.status).toMatchObject({
			state: "error",
			detail: expect.stringContaining("installer unavailable"),
		})
		expect(test.updater.checkForUpdates).toHaveBeenCalledOnce()
		await expect(test.instance.install()).rejects.toThrow("No downloaded update")

		await test.instance.checkNow()
		expect(test.updater.checkForUpdates).toHaveBeenCalledTimes(2)
		test.instance.dispose()
	})

	it("restarts the solver and rejects when the installer throws", async () => {
		let stateAtRestart: string | undefined
		const test = coordinator({
			restartSolver: vi.fn(async () => {
				stateAtRestart = test.instance.status.state
			}),
		})
		test.updater.quitAndInstall.mockImplementation(() => {
			throw new Error("signature rejected")
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow("signature rejected")
		expect(test.options.restartSolver).toHaveBeenCalledOnce()
		// A solver start is refused while an install is in progress, so the state has to leave it first.
		expect(stateAtRestart).toBe("error")
		expect(test.store.value.receipt).toBeUndefined()
		expect(test.options.notify).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("restarts the solver and notifies when the installer fails after it took the update", async () => {
		const test = coordinator()
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await test.instance.install()
		expect(test.instance.status.state).toBe("installing")
		expect(test.options.restartSolver).not.toHaveBeenCalled()

		// macOS: Squirrel fetches and checks the update only after quitAndInstall, and fails later.
		test.updater.emit("error", new Error("code signature did not validate"))
		await vi.waitFor(() => expect(test.options.notify).toHaveBeenCalledOnce())
		expect(test.options.notify).toHaveBeenCalledWith(
			"Simplex update was not installed",
			expect.stringContaining("code signature did not validate"),
		)
		expect(test.options.restartSolver).toHaveBeenCalledOnce()
		expect(test.store.value.receipt).toBeUndefined()
		expect(test.instance.status).toMatchObject({
			state: "error",
			targetVersion: "0.17.0",
			detail: expect.stringContaining("code signature did not validate"),
		})
		test.instance.dispose()
	})

	it("notifies about an installer failure that arrives just after the install was handed over", async () => {
		const test = coordinator()
		test.updater.quitAndInstall.mockImplementation(() => {
			queueMicrotask(() => test.updater.emit("error", new Error("installer exited")))
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await test.instance.install()
		await vi.waitFor(() => expect(test.options.restartSolver).toHaveBeenCalledOnce())
		await vi.waitFor(() =>
			expect(test.options.notify).toHaveBeenCalledExactlyOnceWith(
				"Simplex update was not installed",
				expect.stringContaining("installer exited"),
			),
		)
		test.instance.dispose()
	})

	it("still restarts the solver when clearing the install record fails", async () => {
		let rejectRecoveryWrite = false
		const store = new MemoryUpdateStore()
		vi.spyOn(store, "write").mockImplementation((value) => {
			if (rejectRecoveryWrite && !value.receipt) throw new Error("disk unavailable")
			store.value = structuredClone(value)
		})
		const test = coordinator({ store })
		test.updater.quitAndInstall.mockImplementation(() => {
			rejectRecoveryWrite = true
			test.updater.emit("error", new Error("signature rejected"))
		})

		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow("the install record could not be cleared: disk unavailable")
		expect(test.options.restartSolver).toHaveBeenCalledOnce()
		expect(test.instance.status).toMatchObject({ state: "error", detail: expect.stringContaining("disk unavailable") })
		test.instance.dispose()
	})

	it("says so when the solver it stopped cannot be restarted", async () => {
		const test = coordinator({
			restartSolver: vi.fn(async () => {
				throw new Error("no protected key")
			}),
		})
		test.updater.quitAndInstall.mockImplementation(() => {
			throw new Error("signature rejected")
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow("the solver could not be restarted: no protected key")
		expect(test.instance.status.detail).toContain("the solver could not be restarted")
		test.instance.dispose()
	})

	it.each([
		[{ state: "stopped" as const, detail: "absent" }, "The solver is stopped"],
		[{ state: "starting" as const, pid: 42 }, "The solver is starting"],
		[{ state: "stopping" as const, pid: 42 }, "The solver is stopping"],
		[{ state: "unreachable" as const, detail: "timed out" }, "The solver is unreachable"],
		[{ ...idleSolver(), pid: undefined }, "did not report its process id"],
	])("does not install or restart over a solver it cannot stop and watch exit: %o", async (solver, reason) => {
		const test = coordinator({ probeSolver: vi.fn(async () => solver) })
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await expect(test.instance.install()).rejects.toThrow(reason)
		expect(test.instance.status.state).toBe("ready")
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.options.restartSolver).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("reports an unsuccessful post-update version check once and returns to a normal check", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: { fromVersion: "0.16.2", targetVersion: "0.17.0", installAttemptedAt: 2 },
		})
		const test = coordinator({ appVersion: "0.17.0", store })
		test.instance.start()
		await vi.waitFor(() => expect(test.options.notify).toHaveBeenCalledOnce())
		expect(test.options.notify).toHaveBeenCalledWith(
			"Simplex update needs attention",
			expect.stringContaining("did not start a matching solver"),
		)
		expect(store.value.receipt).toBeUndefined()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalledOnce())
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		await test.instance.checkNow()
		expect(test.options.notify).toHaveBeenCalledOnce()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("checks the last install once when two checks overlap", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: { fromVersion: "0.16.2", targetVersion: "0.17.0", installAttemptedAt: 2 },
		})
		let probed!: (status: ReturnType<typeof idleSolver>) => void
		const probeSolver = vi.fn(
			() =>
				new Promise<ReturnType<typeof idleSolver>>((resolve) => {
					probed = resolve
				}),
		)
		const test = coordinator({ appVersion: "0.17.0", store, probeSolver })
		test.instance.start()
		const second = test.instance.checkNow()
		probed(idleSolver())
		await second
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalled())
		expect(probeSolver).toHaveBeenCalledOnce()
		expect(test.options.notify).toHaveBeenCalledOnce()
		test.instance.dispose()
	})

	it("drops the old channel's check when the channel changes while the last install is checked", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: { fromVersion: "0.16.2", targetVersion: "0.17.0", installAttemptedAt: 2 },
		})
		let probed!: (status: ReturnType<typeof idleSolver>) => void
		const probeSolver = vi.fn(
			() =>
				new Promise<ReturnType<typeof idleSolver>>((resolve) => {
					probed = resolve
				}),
		)
		const test = coordinator({ appVersion: "0.17.0", store, probeSolver })
		const channels: Array<string | null> = []
		test.updater.checkForUpdates.mockImplementation(async () => {
			channels.push(test.updater.channel)
			return undefined
		})
		test.instance.start()
		test.instance.setChannel("beta")
		probed({ ...idleSolver(), version: "0.17.0" })
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalled())
		await new Promise((resolve) => setTimeout(resolve, 0))
		expect(channels).toEqual(["beta"])
		expect(test.instance.status.channel).toBe("beta")
		test.instance.dispose()
	})

	it("looks once more after the deadline, for a machine that slept through it", async () => {
		const probe = vi.fn(async () => ({ state: "spawnable" }))
		await expect(waitForSolverExit({ pid: 42, timeoutMs: 0, probe, processExists: () => false })).resolves.toBe(true)
		expect(probe).toHaveBeenCalledOnce()
	})

	it("waits for both socket removal and PID exit", async () => {
		const probes = [{ state: "stopping" }, { state: "spawnable" }, { state: "spawnable" }]
		let alive = true
		let index = 0
		await expect(
			waitForSolverExit({
				pid: 42,
				timeoutMs: 100,
				probe: async () => probes[Math.min(index++, probes.length - 1)],
				processExists: () => alive,
				delay: async () => {
					alive = false
				},
			}),
		).resolves.toBe(true)
	})

	it("uses socket release as the exit proof for a legacy solver without a reported PID", async () => {
		await expect(
			waitForSolverExit({
				timeoutMs: 100,
				probe: vi.fn().mockResolvedValueOnce({ state: "stopping" }).mockResolvedValue({ state: "spawnable" }),
				processExists: () => {
					throw new Error("no PID should be probed")
				},
				delay: async () => {},
			}),
		).resolves.toBe(true)
	})
})
