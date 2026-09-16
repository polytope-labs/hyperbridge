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
	checkForUpdates = vi.fn(async () => undefined)
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
		work: { queuedEvaluations: 0, evaluating: 0, queuedFills: 0, activeFills: 0, retractions: 0 },
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
		waitForExit: vi.fn(async () => true),
		onChange: vi.fn(),
		notify: vi.fn(),
		...overrides,
	}
	return { updater, store, options, instance: new UpdateCoordinator(options) }
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
				work: { queuedEvaluations: 0, evaluating: 0, queuedFills: 0, activeFills: 1, retractions: 0 },
			})),
		})
		test.instance.start()
		test.updater.emit("update-downloaded", { version: "0.17.0" })
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
		test.updater.emit("update-downloaded", { version: "0.17.0" })
		await vi.waitFor(() => expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true))
		expect(order).toEqual(["stop", "exit", "install"])
		expect(test.store.value.receipt?.installAttemptedAt).toEqual(expect.any(Number))
		test.instance.dispose()
	})

	it("defers without force-installing when graceful exit never completes", async () => {
		const test = coordinator({ waitForExit: vi.fn(async () => false) })
		test.instance.start()
		test.updater.emit("update-downloaded", { version: "0.17.0" })
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
		await vi.waitFor(() => expect(test.updater.quitAndInstall).toHaveBeenCalledWith(false, true))
		expect(test.updater.checkForUpdates).not.toHaveBeenCalled()
		test.instance.dispose()
	})

	it("keeps a staged update retryable when the installer call fails", async () => {
		const scheduled: Array<() => void> = []
		const test = coordinator({
			setTimeout: ((callback: () => void) => {
				scheduled.push(callback)
				return { unref: vi.fn() } as unknown as NodeJS.Timeout
			}) as typeof setTimeout,
		})
		test.updater.quitAndInstall.mockImplementation(() => {
			throw new Error("installer unavailable")
		})
		test.instance.start()
		test.updater.emit("update-downloaded", { version: "0.17.0" })
		await vi.waitFor(() => expect(test.instance.status.state).toBe("deferred"))
		expect(test.store.value.receipt?.installAttemptedAt).toBeUndefined()
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
				work: { queuedEvaluations: 0, evaluating: 0, queuedFills: 1, activeFills: 0, retractions: 0 },
			})),
		})
		test.instance.start()
		await vi.waitFor(() => expect(test.options.notify).toHaveBeenCalledTimes(1))
		expect(store.value.lastNagAt).toBe(now)
		test.instance.dispose()
	})

	it("reports an unsuccessful post-update version check", async () => {
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
		await vi.waitFor(() => expect(test.instance.status.state).toBe("error"))
		expect(test.options.notify).toHaveBeenCalledWith(
			"Simplex update needs attention",
			expect.stringContaining("did not start a matching solver"),
		)
		expect(store.value.receipt).toBeDefined()
		expect(test.options.requestSolverStop).not.toHaveBeenCalled()
		expect(test.updater.quitAndInstall).not.toHaveBeenCalled()
		await test.instance.checkNow()
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
})
