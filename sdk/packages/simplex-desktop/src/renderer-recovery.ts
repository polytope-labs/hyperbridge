/** Recovers the dashboard only. It never controls the detached solver's lifetime. */
export class RendererRecovery {
	private autoReloaded = false
	private pending?: Promise<void>
	private prompting?: Promise<void>
	private crashObserved = false
	private readonly crashObservers = new Set<() => void>()

	constructor(
		private readonly options: {
			canRecover(): boolean
			reload(): Promise<void>
			prompt(): Promise<boolean>
			report(error: Error): void
		},
	) {}

	/** Linux can reject navigation before delivering the renderer's exit event. */
	async loadInitial(load: () => Promise<void>): Promise<void> {
		try {
			await load()
		} catch (error) {
			if (!this.crashObserved) {
				await new Promise<void>((resolve) => {
					const finish = () => {
						clearTimeout(timer)
						this.crashObservers.delete(finish)
						resolve()
					}
					// Bound the correlation window: an unrelated load error must still surface.
					const timer = setTimeout(finish, 1000)
					this.crashObservers.add(finish)
				})
			}
			if (!this.crashObserved) throw error
		}
	}

	async crashed(reason: string, exitCode: number): Promise<void> {
		if (reason === "clean-exit" || !this.options.canRecover()) return
		this.crashObserved = true
		for (const finish of this.crashObservers) finish()
		this.options.report(new Error(`Dashboard renderer exited: ${reason} (code ${exitCode})`))
		if (!this.autoReloaded) {
			this.autoReloaded = true
			return this.show()
		}
		if (!this.prompting) {
			this.prompting = (async () => {
				// Chromium must finish tearing down the old renderer before any recovery UI.
				await new Promise<void>((resolve) => setImmediate(resolve))
				if (this.options.canRecover() && (await this.options.prompt())) await this.show()
			})()
				.catch((error) => this.options.report(error))
				.finally(() => {
					this.prompting = undefined
				})
		}
		await this.prompting
	}

	/** Show Simplex is also an explicit retry after an operator dismisses recovery. */
	show(): Promise<void> {
		if (this.pending) return this.pending
		this.pending = (async () => {
			await new Promise<void>((resolve) => setImmediate(resolve))
			if (this.options.canRecover()) await this.options.reload()
		})()
			.catch((error) => this.options.report(error))
			.finally(() => {
				this.pending = undefined
			})
		return this.pending
	}
}
