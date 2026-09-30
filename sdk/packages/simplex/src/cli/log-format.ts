/** How the CLI renders its own log records to stdout. */
export const LOG_FORMATS = ["pretty", "json"] as const

export type LogFormat = (typeof LOG_FORMATS)[number]

/** What a terminal user has always got, so no flag must ever change it. */
export const DEFAULT_LOG_FORMAT: LogFormat = "pretty"

const FLAG = "--log-format"

function isLogFormat(value: string | undefined): value is LogFormat {
	return value !== undefined && (LOG_FORMATS as readonly string[]).includes(value)
}

/**
 * Reads `--log-format` straight out of raw argv.
 *
 * The console sink is registered while `bin/simplex.ts` is still evaluating —
 * before commander has parsed anything — because records emitted during parsing
 * and command setup need somewhere to go. So the format cannot come from the
 * parsed options, and this reads argv itself.
 *
 * Commander still declares the flag: it owns `--help` and rejects an unknown
 * value. This only has to agree with it on the values that are accepted, so
 * anything else falls back to the default rather than throwing here, before
 * commander can print its own message.
 */
export function logFormatFromArgv(argv: readonly string[]): LogFormat {
	let found: LogFormat = DEFAULT_LOG_FORMAT
	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i]
		// Commander stops parsing options at `--`; a scan that did not would read a
		// filler's own operands as flags.
		if (arg === "--") break
		const value = arg === FLAG ? argv[i + 1] : arg.startsWith(`${FLAG}=`) ? arg.slice(FLAG.length + 1) : undefined
		// Last wins, as it does in commander when an option is repeated.
		if (isLogFormat(value)) found = value
	}
	return found
}
