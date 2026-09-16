import type { SolverStatus } from "./solver-supervisor"
import { solverIsIdle, solverVersion } from "./solver-supervisor"
import type { UpdateChannel, UpdatePreferences, UpdateReceipt, UpdateStore } from "./update-store"

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1_000
const IDLE_RETRY_MS = 15_000
const STALE_AFTER_MS = 24 * 60 * 60 * 1_000
const NAG_INTERVAL_MS = 24 * 60 * 60 * 1_000
const EXIT_TIMEOUT_MS = 10 * 60 * 1_000

export type UpdateState =
	| "disabled"
	| "idle"
	| "checking"
	| "downloading"
	| "waiting-for-idle"
	| "stopping-solver"
	| "installing"
	| "deferred"
	| "error"

export interface UpdateStatus {
	state: UpdateState
	channel: UpdateChannel
	targetVersion?: string
	progress?: number
	detail?: string
}

export interface UpdaterAdapter {
	autoDownload: boolean
	autoInstallOnAppQuit: boolean
	allowPrerelease: boolean
	allowDowngrade: boolean
	channel: string | null
	on(event: string, listener: (...args: any[]) => void): this
	removeListener(event: string, listener: (...args: any[]) => void): this
	checkForUpdates(): Promise<{ downloadPromise?: Promise<unknown> | null } | null | undefined>
	quitAndInstall(isSilent?: boolean, isForceRunAfter?: boolean): void
}

export interface ExitWaitOptions {
	pid?: number
	timeoutMs?: number
	probe: () => Promise<{ state: string }>
	processExists?: (pid: number) => boolean
	delay?: (milliseconds: number) => Promise<void>
}

export async function waitForSolverExit(options: ExitWaitOptions): Promise<boolean> {
	const processExists =
		options.processExists ??
		((pid: number) => {
			try {
				process.kill(pid, 0)
				return true
			} catch (error) {
				return (error as NodeJS.ErrnoException).code === "EPERM"
			}
		})
	const delay =
		options.delay ?? ((milliseconds: number) => new Promise((resolve) => setTimeout(resolve, milliseconds)))
	const deadline = Date.now() + (options.timeoutMs ?? EXIT_TIMEOUT_MS)
	while (Date.now() < deadline) {
		const health = await options.probe().catch(() => ({ state: "unavailable" }))
		const socketGone = health.state === "spawnable"
		if (socketGone && (options.pid === undefined || !processExists(options.pid))) return true
		await delay(250)
	}
	return false
}

export class UpdateCoordinator {
	private current: UpdateStatus
	private preferences: UpdatePreferences
	private checkTimer?: NodeJS.Timeout
	private idleTimer?: NodeJS.Timeout
	private installing = false
	private installAttempt?: Promise<void>
	private checkAttempt?: Promise<void>
	private checkQueued = false
	private activeCheckChannel?: UpdateChannel
	private started = false
	private readonly downloadChannels = new Map<string, UpdateChannel>()

	private readonly listeners: Record<string, (...args: any[]) => void>

	constructor(
		private readonly options: {
			updater: UpdaterAdapter
			packaged: boolean
			appVersion: string
			store: UpdateStore
			probeSolver: () => Promise<SolverStatus>
			requestSolverStop: () => Promise<void>
			waitForExit: (pid: number) => Promise<boolean>
			onChange: (status: UpdateStatus) => void
			notify: (title: string, body: string) => void
			now?: () => number
			setInterval?: typeof setInterval
			setTimeout?: typeof setTimeout
		},
	) {
		this.preferences = options.store.read()
		this.current = {
			state: options.packaged ? "idle" : "disabled",
			channel: this.preferences.channel,
		}
		this.listeners = {
			"update-available": (info: { version?: string }) => {
				const channel = this.activeCheckChannel
				if (!channel) return this.fail(new Error("Available update did not match an active check"))
				if (!info.version) {
					if (channel === this.preferences.channel)
						this.fail(new Error("Available update did not include a version"))
					return
				}
				this.downloadChannels.set(info.version, channel)
				if (channel === this.preferences.channel) {
					this.setStatus({ state: "downloading", targetVersion: info.version })
				}
			},
			"update-not-available": () => {
				if (this.activeCheckChannel === this.preferences.channel && !this.preferences.receipt) {
					this.setStatus({ state: "idle" })
				}
			},
			"download-progress": (progress: { percent?: number }) => {
				const targetVersion = this.current.targetVersion
				if (!targetVersion || this.downloadChannels.get(targetVersion) !== this.preferences.channel) return
				this.setStatus({
					state: "downloading",
					targetVersion,
					progress: typeof progress.percent === "number" ? progress.percent : undefined,
				})
			},
			"update-downloaded": (info: { version?: string }) =>
				void this.updateDownloaded(info.version).catch((error) => this.fail(error)),
			error: (error: unknown) => {
				if (this.activeCheckChannel && this.activeCheckChannel !== this.preferences.channel) return
				this.preferences.receipt ? this.defer(error) : this.fail(error)
			},
		}
	}

	get status(): UpdateStatus {
		return this.current
	}

	start(): void {
		if (this.started || !this.options.packaged) return
		this.started = true
		this.configureUpdater()
		for (const [event, listener] of Object.entries(this.listeners)) this.options.updater.on(event, listener)
		void this.resumeUpdateLifecycle().catch((error) => this.fail(error))
		this.checkTimer = (this.options.setInterval ?? setInterval)(() => void this.checkNow(), CHECK_INTERVAL_MS)
		this.checkTimer.unref?.()
	}

	dispose(): void {
		if (this.checkTimer) clearInterval(this.checkTimer)
		if (this.idleTimer) clearTimeout(this.idleTimer)
		this.checkTimer = undefined
		this.idleTimer = undefined
		this.checkQueued = false
		this.activeCheckChannel = undefined
		this.downloadChannels.clear()
		for (const [event, listener] of Object.entries(this.listeners))
			this.options.updater.removeListener(event, listener)
		this.started = false
	}

	async checkNow(): Promise<void> {
		if (!this.options.packaged || this.installing) return
		let checkChannel: UpdateChannel | undefined
		try {
			if (this.preferences.receipt?.installAttemptedAt) {
				await this.verifyPreviousInstall()
				return
			}
			if (this.preferences.receipt) {
				await this.tryInstall()
				return
			}
			if (this.checkAttempt) {
				this.checkQueued = true
				await this.checkAttempt.catch(() => {})
				return
			}
			const channel = this.preferences.channel
			checkChannel = channel
			const attempt = this.runUpdateCheck(channel)
			this.checkAttempt = attempt
			try {
				await attempt
			} finally {
				if (this.checkAttempt === attempt) this.checkAttempt = undefined
				if (this.activeCheckChannel === channel) this.activeCheckChannel = undefined
				if (this.started && this.checkQueued) {
					this.checkQueued = false
					void this.checkNow()
				}
			}
		} catch (error) {
			if (!checkChannel || checkChannel === this.preferences.channel) this.fail(error)
		}
	}

	setChannel(channel: UpdateChannel): void {
		if (this.preferences.channel === channel || this.installing) return
		try {
			const receipt = this.preferences.receipt?.installAttemptedAt ? this.preferences.receipt : undefined
			this.preferences = { ...this.preferences, channel, receipt, lastNagAt: undefined }
			this.options.store.write(this.preferences)
			this.configureUpdater()
			this.setStatus({ state: "idle", channel })
			void this.checkNow()
		} catch (error) {
			this.fail(error)
		}
	}

	private configureUpdater(): void {
		const beta = this.preferences.channel === "beta"
		this.options.updater.autoDownload = true
		// Load-bearing: ordinary app quit must never install over a detached solver.
		this.options.updater.autoInstallOnAppQuit = false
		this.options.updater.allowPrerelease = beta
		// electron-updater's channel setter enables downgrades. Set it first, then
		// restore the invariant that changing channels never installs an older build.
		this.options.updater.channel = beta ? "beta" : "latest"
		this.options.updater.allowDowngrade = false
	}

	private async runUpdateCheck(channel: UpdateChannel): Promise<void> {
		this.activeCheckChannel = channel
		this.setStatus({ state: "checking", channel })
		const result = await this.options.updater.checkForUpdates()
		if (result?.downloadPromise) await result.downloadPromise
	}

	private async resumeUpdateLifecycle(): Promise<void> {
		if (this.preferences.receipt?.installAttemptedAt) {
			await this.verifyPreviousInstall()
			if (this.current.state === "error") return
		}
		if (this.preferences.receipt) {
			this.setStatus({
				state: "waiting-for-idle",
				targetVersion: this.preferences.receipt.targetVersion,
			})
			await this.tryInstall()
			return
		}
		await this.checkNow()
	}

	private async verifyPreviousInstall(): Promise<void> {
		const receipt = this.preferences.receipt
		if (!receipt?.installAttemptedAt) return
		let solver: SolverStatus
		try {
			solver = await this.options.probeSolver()
		} catch (error) {
			return this.reportFailedInstall(receipt, error)
		}
		if (this.options.appVersion === receipt.targetVersion && solverVersion(solver) === this.options.appVersion) {
			this.preferences = { ...this.preferences, receipt: undefined, lastNagAt: undefined }
			this.options.store.write(this.preferences)
			this.setStatus({ state: "idle" })
			return
		}
		this.reportFailedInstall(receipt)
	}

	private reportFailedInstall(receipt: UpdateReceipt, cause?: unknown): void {
		const suffix = cause ? `: ${cause instanceof Error ? cause.message : String(cause)}` : ""
		this.setStatus({
			state: "error",
			targetVersion: receipt.targetVersion,
			detail: `Update ${receipt.targetVersion} did not start a matching healthy solver${suffix}`,
		})
		this.options.notify(
			"Simplex update needs attention",
			`The update to ${receipt.targetVersion} did not start a matching solver. Open Simplex and review the solver log.`,
		)
	}

	private async updateDownloaded(version: string | undefined): Promise<void> {
		if (!version) return this.fail(new Error("Downloaded update did not include a version"))
		const channel = this.downloadChannels.get(version)
		this.downloadChannels.delete(version)
		if (!channel) return this.fail(new Error(`Downloaded update ${version} did not match an active check`))
		if (channel !== this.preferences.channel) return
		const receipt: UpdateReceipt = {
			fromVersion: this.options.appVersion,
			targetVersion: version,
			downloadedAt: this.options.now?.() ?? Date.now(),
		}
		this.preferences = { ...this.preferences, receipt, lastNagAt: undefined }
		this.options.store.write(this.preferences)
		this.setStatus({ state: "waiting-for-idle", targetVersion: version })
		await this.tryInstall()
	}

	private tryInstall(): Promise<void> {
		if (this.installAttempt) return this.installAttempt
		const attempt = this.runInstallAttempt()
		this.installAttempt = attempt
		return attempt.finally(() => {
			if (this.installAttempt === attempt) this.installAttempt = undefined
		})
	}

	private async runInstallAttempt(): Promise<void> {
		if (this.installing) return
		const receipt = this.preferences.receipt
		if (!receipt) return
		const solver = await this.options.probeSolver().catch((error) => {
			this.defer(error)
			return undefined
		})
		if (!solver) return this.scheduleIdleRetry()
		// A channel change can discard the receipt while the idle probe is pending.
		if (this.preferences.receipt !== receipt) return

		if (solver.state === "stopped") {
			return this.install(receipt)
		}
		if (solver.state !== "setup" && solver.state !== "running" && solver.state !== "paused") {
			this.defer(new Error(`Solver is ${solver.state}; update remains staged`))
			return this.scheduleIdleRetry()
		}
		if ((solver.state === "running" || solver.state === "paused") && !solverIsIdle(solver)) {
			this.setStatus({ state: "waiting-for-idle", targetVersion: receipt.targetVersion })
			this.maybeNag(receipt)
			return this.scheduleIdleRetry()
		}
		if (!solver.pid) {
			this.defer(new Error("Solver did not report its process id"))
			return this.scheduleIdleRetry()
		}

		this.installing = true
		this.setStatus({ state: "stopping-solver", targetVersion: receipt.targetVersion })
		try {
			await this.options.requestSolverStop()
			if (!(await this.options.waitForExit(solver.pid))) {
				throw new Error("Solver did not exit after its graceful stop; update remains staged")
			}
			await this.install(receipt)
		} catch (error) {
			this.installing = false
			this.defer(error)
			this.scheduleIdleRetry()
		}
	}

	private async install(receipt: UpdateReceipt): Promise<void> {
		this.installing = true
		try {
			const attempted: UpdateReceipt = {
				...receipt,
				installAttemptedAt: this.options.now?.() ?? Date.now(),
			}
			this.preferences = { ...this.preferences, receipt: attempted }
			this.options.store.write(this.preferences)
			this.setStatus({ state: "installing", targetVersion: receipt.targetVersion })
			this.options.updater.quitAndInstall(false, true)
		} catch (error) {
			this.installing = false
			this.preferences = { ...this.preferences, receipt }
			try {
				this.options.store.write(this.preferences)
			} catch (restoreError) {
				const installDetail = error instanceof Error ? error.message : String(error)
				const restoreDetail = restoreError instanceof Error ? restoreError.message : String(restoreError)
				this.fail(
					new Error(
						`The update could not start (${installDetail}) and its retry receipt could not be restored (${restoreDetail})`,
					),
				)
				this.scheduleIdleRetry()
				return
			}
			this.defer(error)
			this.scheduleIdleRetry()
		}
	}

	private scheduleIdleRetry(): void {
		if (this.idleTimer || this.installing) return
		this.idleTimer = (this.options.setTimeout ?? setTimeout)(() => {
			this.idleTimer = undefined
			void this.tryInstall()
		}, IDLE_RETRY_MS)
		this.idleTimer.unref?.()
	}

	private maybeNag(receipt: UpdateReceipt): void {
		const now = this.options.now?.() ?? Date.now()
		if (now - receipt.downloadedAt < STALE_AFTER_MS) return
		if (this.preferences.lastNagAt && now - this.preferences.lastNagAt < NAG_INTERVAL_MS) return
		this.preferences = { ...this.preferences, lastNagAt: now }
		this.options.store.write(this.preferences)
		this.options.notify(
			"Simplex update is waiting",
			`Version ${receipt.targetVersion} has been ready for more than a day. Pause new fills to create a safe update window.`,
		)
	}

	private defer(error: unknown): void {
		const detail = error instanceof Error ? error.message : String(error)
		this.setStatus({ state: "deferred", targetVersion: this.preferences.receipt?.targetVersion, detail })
	}

	private fail(error: unknown): void {
		const detail = error instanceof Error ? error.message : String(error)
		this.setStatus({ state: "error", targetVersion: this.current.targetVersion, detail })
	}

	private setStatus(next: Omit<UpdateStatus, "channel"> & { channel?: UpdateChannel }): void {
		this.current = { ...next, channel: next.channel ?? this.preferences.channel }
		this.options.onChange(this.current)
	}
}
