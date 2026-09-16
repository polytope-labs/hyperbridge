interface DesktopArgumentOptions {
	hidden?: boolean
}

interface ElectronChildProcess {
	exitCode: number | null
	signalCode: NodeJS.Signals | null
	pid?: number
	once(event: "exit", listener: () => void): unknown
}

export function desktopArguments(
	packageRoot: string,
	userDataDir: string,
	options: DesktopArgumentOptions = {},
): string[] {
	const args = [packageRoot, `--user-data-dir=${userDataDir}`]
	if (options.hidden) args.push("--hidden")
	return args
}

/**
 * Playwright adds this flag to its Linux Electron launches. Direct launches in
 * the E2E harness must do the same because CI cannot install Electron's
 * chrome-sandbox helper as a root-owned setuid binary.
 */
export function directElectronArguments(
	packageRoot: string,
	userDataDir: string,
	platform: NodeJS.Platform = process.platform,
): string[] {
	const args = desktopArguments(packageRoot, userDataDir)
	if (platform === "linux") args.unshift("--no-sandbox")
	return args
}

/**
 * Playwright's Electron "close" event waits for stdio pipes as well as the
 * process. A detached Windows solver can inherit those pipes, so lifecycle
 * tests must wait for the launcher process itself.
 */
export function electronProcessExit(child: ElectronChildProcess, timeoutMs = 30_000): Promise<void> {
	if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve()
	return new Promise((resolveExit, reject) => {
		const timer = setTimeout(() => reject(new Error(`Electron ${child.pid ?? "process"} did not exit`)), timeoutMs)
		child.once("exit", () => {
			clearTimeout(timer)
			resolveExit()
		})
	})
}
