import { existsSync } from "fs"
import { join, resolve } from "path"

export const DEFAULT_CONFIG_FILENAME = "filler-config.toml"

/**
 * Locates a config when `run` is invoked without -c: the working directory's
 * filler-config.toml first, then $SIMPLEX_HOME/config.toml.
 */
export function discoverConfigPath(cwd = process.cwd()): string | undefined {
	const local = resolve(cwd, DEFAULT_CONFIG_FILENAME)
	if (existsSync(local)) return local

	const home = process.env.SIMPLEX_HOME
	if (home) {
		const homeConfig = join(home, "config.toml")
		if (existsSync(homeConfig)) return homeConfig
	}
	return undefined
}
