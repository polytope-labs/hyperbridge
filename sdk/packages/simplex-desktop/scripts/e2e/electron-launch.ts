interface DesktopArgumentOptions {
	hidden?: boolean
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
