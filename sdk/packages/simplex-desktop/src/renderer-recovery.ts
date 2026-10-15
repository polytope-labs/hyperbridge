/** Recovers the dashboard only. It never controls the detached solver's lifetime. */
export class RendererRecovery {
	private autoReloaded = false
	private pending?: Promise<void>
	private prompting?: Promise<void>

	constructor(
		private readonly options: {
			canRecover(): boolean
			reload(): Promise<void>
			prompt(): Promise<boolean>
			report(error: Error): void
		},
	) {}

	async crashed(reason: string, exitCode: number): Promise<void> {
		if (reason === "clean-exit" || !this.options.canRecover()) return
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
