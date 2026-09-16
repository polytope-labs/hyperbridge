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
		requestSolverStop: vi.fn(async () => undefined),
		restartSolver: vi.fn(async () => undefined),
		waitForExit: vi.fn(async () => true),
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

	it("downloads while active work continues and waits without stopping", async () => {
		const test = coordinator({
			probeSolver: vi.fn(async () => ({
				...idleSolver(),
				work: {
					queuedEvaluations: 0,
					evaluating: 0,
					queuedFills: 0,
					activeFills: 1,
					retractions: 0,
					rebalancing: 0,
				},
			})),
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.instance.status.state).toBe("waiting-for-idle"))
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
				return true
			}),
		})
		test.updater.quitAndInstall.mockImplementation(() => order.push("install"))
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true))
		expect(order).toEqual(["stop", "exit", "install"])
		expect(test.store.value.receipt?.installAttemptedAt).toEqual(expect.any(Number))
		test.instance.dispose()
	})

	it("defers without force-installing when graceful exit never completes", async () => {
		const test = coordinator({ waitForExit: vi.fn(async () => false) })
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.instance.status.state).toBe("deferred"))
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
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

	it("discards a staged download when the operator changes channels", () => {
		const store = new MemoryUpdateStore({
			channel: "beta",
			receipt: { fromVersion: "0.16.2", targetVersion: "0.17.0-beta.1", downloadedAt: 1 },
		})
		const test = coordinator({ store })
		test.instance.setChannel("stable")
		expect(store.value.receipt).toBeUndefined()
		expect(test.updater.allowDowngrade).toBe(false)
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
		expect(store.value.receipt).toBeUndefined()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("cancels a staged install when the channel changes during its idle probe", async () => {
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
		await vi.waitFor(() => expect(probeSolver).toHaveBeenCalled())

		test.instance.setChannel("beta")
		resolveProbe(idleSolver())
		await new Promise((resolve) => setTimeout(resolve, 0))

		expect(test.store.value.receipt).toBeUndefined()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("clears an attempted receipt only after app and solver versions match", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: {
				fromVersion: "0.16.2",
				targetVersion: "0.17.0",
				downloadedAt: 1,
				installAttemptedAt: 2,
			},
		})
		const test = coordinator({
			appVersion: "0.17.0",
			store,
			probeSolver: vi.fn(async () => ({ ...idleSolver(), version: "0.17.0" })),
		})
		test.instance.start()
		await vi.waitFor(() => expect(store.value.receipt).toBeUndefined())
		test.instance.dispose()
	})

	it("accepts a healthy matching setup solver after an update", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: {
				fromVersion: "0.16.2",
				targetVersion: "0.17.0",
				downloadedAt: 1,
				installAttemptedAt: 2,
			},
		})
		const test = coordinator({
			appVersion: "0.17.0",
			store,
			probeSolver: vi.fn(async () => ({ state: "setup" as const, pid: 42, version: "0.17.0" })),
		})
		test.instance.start()
		await vi.waitFor(() => expect(store.value.receipt).toBeUndefined())
		expect(test.updater.checkForUpdates).toHaveBeenCalled()
		test.instance.dispose()
	})

	it("resumes a staged download after the desktop app restarts", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: {
				fromVersion: "0.16.2",
				targetVersion: "0.17.0",
				downloadedAt: 1,
			},
		})
		const test = coordinator({ store })
		test.instance.start()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalled())
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true))
		test.instance.dispose()
	})

	it("restarts the solver and keeps the update retryable when electron-updater emits an installer error", async () => {
		const scheduled: Array<() => void> = []
		const test = coordinator({
			setTimeout: ((callback: () => void) => {
				scheduled.push(callback)
				return { unref: vi.fn() } as unknown as NodeJS.Timeout
			}) as typeof setTimeout,
		})
		test.updater.quitAndInstall.mockImplementation(() => {
			test.updater.emit("error", new Error("installer unavailable"))
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.instance.status.state).toBe("deferred"))
		expect(test.store.value.receipt?.installAttemptedAt).toBeUndefined()
		expect(test.options.restartSolver).toHaveBeenCalledOnce()
		expect(scheduled).toHaveLength(1)
		test.instance.dispose()
	})

	it("still restarts the solver when persisting installer recovery fails", async () => {
		let rejectRecoveryWrite = false
		const store = new MemoryUpdateStore()
		const write = vi.spyOn(store, "write").mockImplementation((value) => {
			if (rejectRecoveryWrite && value.receipt && !value.receipt.installAttemptedAt) {
				throw new Error("disk unavailable")
			}
			store.value = structuredClone(value)
		})
		const test = coordinator({ store })
		test.updater.quitAndInstall.mockImplementation(() => {
			rejectRecoveryWrite = true
			test.updater.emit("error", new Error("signature rejected"))
		})

		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.options.restartSolver).toHaveBeenCalledOnce())
		expect(test.instance.status.state).toBe("error")
		expect(write).toHaveBeenCalled()
		test.instance.dispose()
	})

	it("does not install or restart while the operator has intentionally left the solver stopped", async () => {
		const scheduled: Array<() => void> = []
		const test = coordinator({
			probeSolver: vi.fn(async () => ({ state: "stopped" as const, detail: "absent" })),
			setTimeout: ((callback: () => void) => {
				scheduled.push(callback)
				return { unref: vi.fn() } as unknown as NodeJS.Timeout
			}) as typeof setTimeout,
		})
		test.instance.start()
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.instance.status.state).toBe("deferred"))
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.options.restartSolver).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		expect(scheduled).toHaveLength(1)
		test.instance.dispose()
	})

	it("nags once a staged update has waited a day for active work", async () => {
		const now = 2 * 24 * 60 * 60 * 1_000
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: {
				fromVersion: "0.16.2",
				targetVersion: "0.17.0",
				downloadedAt: 1,
			},
		})
		const test = coordinator({
			store,
			now: () => now,
			probeSolver: vi.fn(async () => ({
				...idleSolver(),
				work: {
					queuedEvaluations: 0,
					evaluating: 0,
					queuedFills: 1,
					activeFills: 0,
					retractions: 0,
					rebalancing: 0,
				},
			})),
		})
		test.instance.start()
		await vi.waitFor(() => expect(test.updater.checkForUpdates).toHaveBeenCalled())
		finishDownload(test.updater, "0.17.0")
		await vi.waitFor(() => expect(test.options.notify).toHaveBeenCalledTimes(1))
		expect(store.value.lastNagAt).toBe(now)
		test.instance.dispose()
	})

	it("reports an unsuccessful post-update version check once and returns to a normal check", async () => {
		const store = new MemoryUpdateStore({
			channel: "stable",
			receipt: {
				fromVersion: "0.16.2",
				targetVersion: "0.17.0",
				downloadedAt: 1,
				installAttemptedAt: 2,
			},
		})
		const test = coordinator({ appVersion: "0.17.0", store })
		test.instance.start()
		await vi.waitFor(() => expect(test.options.notify).toHaveBeenCalledOnce())
		expect(test.options.notify).toHaveBeenCalledWith(
			"Simplex update needs attention",
			expect.stringContaining("did not start a matching solver"),
		)
		expect(store.value.receipt?.installAttemptedAt).toBeUndefined()
		expect(test.updater.checkForUpdates).toHaveBeenCalledOnce()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		await test.instance.checkNow()
		expect(test.options.notify).toHaveBeenCalledOnce()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		test.instance.dispose()
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
