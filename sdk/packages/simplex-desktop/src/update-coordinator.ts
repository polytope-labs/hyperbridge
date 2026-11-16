import type { SolverStatus } from "./solver-supervisor"
import { solverVersion } from "./solver-supervisor"
import type { UpdateChannel, UpdatePreferences, UpdateStore } from "./update-store"

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1_000
const EXIT_TIMEOUT_MS = 10 * 60 * 1_000

/**
 * `ready` is a downloaded update that waits for the operator: `targetVersion` is installed only
 * when they ask for it. `stopping-solver` and `installing` are that install in progress.
 */
export type UpdateState =
	| "disabled"
	| "idle"
	| "checking"
	| "downloading"
	| "ready"
	| "stopping-solver"
	| "installing"
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
	for (;;) {
		const health = await options.probe().catch(() => ({ state: "unavailable" }))
		const socketGone = health.state === "spawnable"
		if (socketGone && (options.pid === undefined || !processExists(options.pid))) return true
		// Read after the probe, so a machine that slept past the deadline still gets one look.
		if (Date.now() >= deadline) return false
		await delay(250)
	}
}

/** An update is stopping the solver or running the installer, so nothing else may start a solver. */
export function updateInstalling(status: UpdateStatus): boolean {
	return status.state === "stopping-solver" || status.state === "installing"
}

export class UpdateCoordinator {
	private current: UpdateStatus
	private preferences: UpdatePreferences
	private checkTimer?: NodeJS.Timeout
	private installAttempt?: Promise<void>
	private checkAttempt?: Promise<void>
	private checkQueued = false
	private activeCheckChannel?: UpdateChannel
	private started = false
	private solverStoppedForInstall = false
	/** Set while `install` will still reject with an installer failure. After that, `notify` reports it. */
	private installRejects = false
	private recovery?: Promise<string>
	private readonly downloadChannels = new Map<string, UpdateChannel>()

	private readonly listeners: Record<string, (...args: any[]) => void>

	constructor(
		private readonly options: {
			updater: UpdaterAdapter
			packaged: boolean
			appVersion: string
			store: UpdateStore
			probeSolver: () => Promise<SolverStatus>
			canRestartSolver: () => boolean
			requestSolverStop: () => Promise<void>
			restartSolver: () => Promise<void>
			/** Resolves once the solver's socket and process are gone. */
			waitForExit: (pid: number) => Promise<void>
			onChange: (status: UpdateStatus) => void
			notify: (title: string, body: string) => void
			now?: () => number
			setInterval?: typeof setInterval
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
				if (this.activeCheckChannel !== this.preferences.channel) return
				this.setStatus({ state: "idle" })
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
			"update-downloaded": (info: { version?: string }) => this.updateDownloaded(info.version),
			error: (error: unknown) => {
				if (this.activeCheckChannel && this.activeCheckChannel !== this.preferences.channel) return
				if (this.current.state === "installing") {
					void this.recoverInstallerFailure(error)
					return
				}
				this.fail(error)
			},
		}
	}

	get status(): UpdateStatus {
		return this.current
	}

	/** Nothing else may touch the updater while an install is in progress. */
	private get installing(): boolean {
		return updateInstalling(this.current)
	}

	start(): void {
		if (this.started || !this.options.packaged) return
		this.started = true
		this.configureUpdater()
		for (const [event, listener] of Object.entries(this.listeners)) this.options.updater.on(event, listener)
		void this.checkNow()
		this.checkTimer = (this.options.setInterval ?? setInterval)(() => void this.checkNow(), CHECK_INTERVAL_MS)
		this.checkTimer.unref?.()
	}

	dispose(): void {
		if (this.checkTimer) clearInterval(this.checkTimer)
		this.checkTimer = undefined
		this.checkQueued = false
		this.activeCheckChannel = undefined
		this.downloadChannels.clear()
		for (const [event, listener] of Object.entries(this.listeners))
			this.options.updater.removeListener(event, listener)
		this.started = false
	}

	/** Checks for an update and downloads it. A downloaded update is kept as it is until it is installed. */
	async checkNow(): Promise<void> {
		if (!this.options.packaged || this.installing || this.current.state === "ready") return
		let checkChannel: UpdateChannel | undefined
		try {
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
			if (!checkChannel || checkChannel !== this.preferences.channel) return
			this.fail(error)
		}
	}

	setChannel(channel: UpdateChannel): void {
		if (this.preferences.channel === channel || this.installing) return
		try {
			// Saved first: a channel that could not be saved is not the one the updater follows.
			const next = { ...this.preferences, channel }
			this.options.store.write(next)
			this.preferences = next
			this.configureUpdater()
			// Leaving `ready` discards an update downloaded on the old channel.
			this.setStatus({ state: "idle", channel })
			void this.checkNow()
		} catch (error) {
			this.fail(error)
		}
	}

	/**
	 * Installs the downloaded update, and is called only when the operator asks for it. The solver
	 * is stopped and its exit proven before the installer runs and relaunches the app.
	 *
	 * Resolves once the installer has taken the update. Rejects when the solver could not be stopped,
	 * which leaves the update downloaded, and when the installer fails at once, which restarts the
	 * solver. An installer failure that arrives after it resolved is reported through `notify`.
	 */
	install(): Promise<void> {
		this.installAttempt ??= this.runInstall().finally(() => {
			this.installAttempt = undefined
		})
		return this.installAttempt
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
		if (this.preferences.receipt) {
			await this.verifyPreviousInstall()
			// The channel changed during the probe: the check queued behind this one follows the new one.
			if (channel !== this.preferences.channel) return
		}
		this.activeCheckChannel = channel
		this.setStatus({ state: "checking", channel })
		const result = await this.options.updater.checkForUpdates()
		if (result?.downloadPromise) await result.downloadPromise
	}

	/** An install was started in the last session: it took if this app and its solver are that version. */
	private async verifyPreviousInstall(): Promise<void> {
		const receipt = this.preferences.receipt
		if (!receipt) return
		let failure: string | undefined
		try {
			const solver = await this.options.probeSolver()
			const installed =
				this.options.appVersion === receipt.targetVersion && solverVersion(solver) === this.options.appVersion
			if (!installed) failure = ""
		} catch (error) {
			failure = `: ${errorMessage(error)}`
		}
		this.preferences = { ...this.preferences, receipt: undefined }
		this.options.store.write(this.preferences)
		if (failure === undefined) return
		this.setStatus({
			state: "error",
			targetVersion: receipt.targetVersion,
			detail: `Update ${receipt.targetVersion} did not start a matching healthy solver${failure}`,
		})
		this.options.notify(
			"Simplex update needs attention",
			`The update to ${receipt.targetVersion} did not start a matching solver. Open Simplex and review the solver log.`,
		)
	}

	private updateDownloaded(version: string | undefined): void {
		if (!version) return this.fail(new Error("Downloaded update did not include a version"))
		const channel = this.downloadChannels.get(version)
		this.downloadChannels.delete(version)
		if (!channel) return this.fail(new Error(`Downloaded update ${version} did not match an active check`))
		if (channel !== this.preferences.channel) return
		this.setStatus({ state: "ready", targetVersion: version })
	}

	private async runInstall(): Promise<void> {
		const version = this.current.state === "ready" ? this.current.targetVersion : undefined
		if (!version) throw new Error("No downloaded update is ready to install")
		const solver = await this.options.probeSolver()
		// A channel change can discard the download while the probe is pending.
		if (this.current.state !== "ready" || this.current.targetVersion !== version) {
			throw new Error("The downloaded update was discarded before it could be installed")
		}
		if (solver.state !== "setup" && solver.state !== "running" && solver.state !== "paused") {
			throw new Error(
				solver.state === "stopped"
					? "The solver is stopped. Restart it, then install the update."
					: `The solver is ${solver.state}. Try again in a moment.`,
			)
		}
		if (!solver.pid) throw new Error("The solver did not report its process id, so its exit cannot be confirmed")
		if (!this.options.canRestartSolver()) {
			throw new Error("Simplex cannot restart the solver right now, so the update was not installed")
		}

		this.setStatus({ state: "stopping-solver", targetVersion: version })
		try {
			await this.stopSolver()
			// A stop cannot be taken back, so there is no deadline: the update goes in once the solver has drained.
			await this.options.waitForExit(solver.pid)
		} catch (error) {
			// The installer was never reached, so the download is offered again unless the updater failed meanwhile.
			if (this.status.state === "stopping-solver") this.setStatus({ state: "ready", targetVersion: version })
			throw error
		}
		this.solverStoppedForInstall = true
		let failure: Promise<string> | undefined
		this.installRejects = true
		try {
			try {
				// The updater failed while the solver was stopping, so there is nothing sound to install.
				if (this.status.state !== "stopping-solver") {
					throw new Error(this.current.detail ?? "The update is no longer available")
				}
				this.preferences = {
					...this.preferences,
					receipt: {
						fromVersion: this.options.appVersion,
						targetVersion: version,
						installAttemptedAt: this.options.now?.() ?? Date.now(),
					},
				}
				this.options.store.write(this.preferences)
				this.setStatus({ state: "installing", targetVersion: version })
				this.options.updater.quitAndInstall(false, true)
			} catch (error) {
				failure = this.recoverInstallerFailure(error)
			}
			// electron-updater reports most installer failures through its error event instead of throwing.
			failure ??= this.recovery
			if (failure) throw new Error(await failure)
		} finally {
			this.installRejects = false
		}
	}

	/** Asks the solver to stop. A request that fails after the solver took it still counts as a stop. */
	private async stopSolver(): Promise<void> {
		try {
			await this.options.requestSolverStop()
		} catch (error) {
			const solver = await this.options.probeSolver().catch(() => undefined)
			if (solver?.state !== "stopping" && solver?.state !== "stopped") throw error
		}
	}

	/**
	 * The installer failed with the solver already stopped. The app stays on this version, so the
	 * solver is started again. The state stays `error` with the reason until the next check.
	 * Resolves to that reason.
	 */
	private recoverInstallerFailure(error: unknown): Promise<string> {
		this.recovery ??= this.runRecovery(error).finally(() => {
			this.recovery = undefined
		})
		return this.recovery
	}

	private async runRecovery(error: unknown): Promise<string> {
		const version = this.current.targetVersion
		const rejected = this.installRejects
		const problems: string[] = []
		const detail = () => [`Update ${version} could not be installed: ${errorMessage(error)}`, ...problems].join("; ")
		this.preferences = { ...this.preferences, receipt: undefined }
		try {
			this.options.store.write(this.preferences)
		} catch (storeError) {
			problems.push(`the install record could not be cleared: ${errorMessage(storeError)}`)
		}
		// Leaves `installing` before the restart, which is refused while an install is in progress.
		this.fail(new Error(detail()))
		if (this.solverStoppedForInstall) {
			this.solverStoppedForInstall = false
			try {
				await this.options.restartSolver()
			} catch (restartError) {
				problems.push(`the solver could not be restarted: ${errorMessage(restartError)}`)
				this.fail(new Error(detail()))
			}
		}
		if (!rejected) this.options.notify("Simplex update was not installed", `${detail()}.`)
		return detail()
	}

	private fail(error: unknown): void {
		this.setStatus({ state: "error", targetVersion: this.current.targetVersion, detail: errorMessage(error) })
	}

	private setStatus(next: Omit<UpdateStatus, "channel"> & { channel?: UpdateChannel }): void {
		this.current = { ...next, channel: next.channel ?? this.preferences.channel }
		this.options.onChange(this.current)
	}
}

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}
