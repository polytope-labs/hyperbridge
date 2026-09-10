import type { Command } from "commander"
import { DEFAULT_CONFIG_FILENAME } from "@/cli/discover-config"

/** Port the local web UI binds when `--ui` names no other one. */
export const DEFAULT_UI_PORT = 8686

/** What `run`'s action handler receives, in the shape `addRunOptions` produces. */
export interface RunOptions {
	config?: string
	dataDir?: string
	watchOnly?: boolean
	/**
	 * A bind address from `--ui <addr>`, `true` from a bare `--ui`, `false` from
	 * `--no-ui`, and absent when neither flag is passed.
	 */
	ui?: string | boolean
	/** A socket path from `--ui-socket <path>`; absent when the flag is not passed. */
	uiSocket?: string
}

/**
 * Declares the flags of `simplex run`. Split out of the bin so tests can parse
 * them: importing the bin runs `program.parse(process.argv)` at module load.
 */
export function addRunOptions(command: Command): Command {
	// The `--ui` placeholder must contain no angle brackets. Commander derives
	// `required` from a `<` *anywhere* in the flags string, and required beats
	// optional — so the earlier `[<[host:]port>]` turned a valueless `--ui` into
	// an "argument missing" error.
	return command
		.option("-c, --config <path>", `Path to TOML configuration file (default: ./${DEFAULT_CONFIG_FILENAME})`)
		.option("-d, --data-dir <path>", "Directory for persistent data storage (bids database, etc.)")
		.option("--watch-only", "Watch-only mode: monitor orders without executing fills", false)
		.option(
			"--ui [host:port]",
			`Bind address for the local web UI (status, pause/resume, price curves); a bare port keeps the host at 127.0.0.1. Unauthenticated; default 127.0.0.1:${DEFAULT_UI_PORT}`,
		)
		.option(
			"--ui-socket <path>",
			"Serve the web UI on a Unix domain socket at <path> instead of a TCP port. The socket is created 0600, so no other local user and no web page can reach it. On Windows this is a named pipe, whose default ACL is weaker (see docs/ai/Decisions.md). For embedding simplex in a desktop application",
		)
		.option("--no-ui", "Disable the local web UI")
}
