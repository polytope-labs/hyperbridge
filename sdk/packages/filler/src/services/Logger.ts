import pino from "pino"

type LoggerOptions = {
	module?: string
}

const level = "info"
const isPretty = "true"

// Base logger
const baseLogger = pino({
	level,
	transport: isPretty
		? {
				target: "pino-pretty",
				options: {
					colorize: true,
					singleLine: true,
					ignore: "pid,hostname,moduleTag",
					messageFormat: "{moduleTag}: {msg}",
				},
			}
		: undefined,
})

export function getLogger(moduleOrOptions?: string | LoggerOptions) {
	const options: LoggerOptions =
		typeof moduleOrOptions === "string" ? { module: moduleOrOptions } : moduleOrOptions || {}
	if (options.module) {
		return baseLogger.child({ moduleTag: `[${options.module}]` })
	}
	return baseLogger
}

export type Logger = ReturnType<typeof getLogger>
