import { existsSync } from "fs"
import { join, resolve } from "path"

export const DEFAULT_CONFIG_FILENAME = "filler-config.toml"

/**
 * Locates a config when `run` is invoked without -c: the working directory's
 * filler-config.toml first, then $SIMPLEX_HOME/config.toml, then an optional
 * application data directory. The last location lets a desktop host supply a
 * stable home without changing the two CLI locations or their precedence.
 */
export function discoverConfigPath(cwd = process.cwd(), appDataDir?: string): string | undefined {
	const local = resolve(cwd, DEFAULT_CONFIG_FILENAME)
	if (existsSync(local)) return local

	const home = process.env.SIMPLEX_HOME
	if (home) {
		const homeConfig = join(home, "config.toml")
		if (existsSync(homeConfig)) return homeConfig
	}

	if (appDataDir) {
		const appConfig = join(appDataDir, DEFAULT_CONFIG_FILENAME)
		if (existsSync(appConfig)) return appConfig
	}
	return undefined
}
