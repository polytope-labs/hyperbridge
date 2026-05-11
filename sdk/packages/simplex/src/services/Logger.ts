import pino, { stdSerializers } from "pino"
export type LogLevel = "trace" | "debug" | "info" | "warn" | "error"

type LoggerOptions = {
	module?: string
}

let logLevel: LogLevel = "info"

let baseLogger: pino.Logger

// Initialize the logger with current config
function initializeLogger() {
	baseLogger = pino({
		level: logLevel,
		serializers: {
			error: stdSerializers.err,
			err: stdSerializers.err,
		},
		transport: {
			target: "pino-pretty",
			options: {
				colorize: true,
				singleLine: true,
				ignore: "pid,hostname,moduleTag",
				messageFormat: "{moduleTag}: {msg}",
			},
		},
	})
}

initializeLogger()

export function configureLogger(level: LogLevel) {
	logLevel = level
	initializeLogger()
}

export function getLogger(moduleOrOptions?: string | LoggerOptions) {
	const options: LoggerOptions =
		typeof moduleOrOptions === "string" ? { module: moduleOrOptions } : moduleOrOptions || {}
	if (options.module) {
		return baseLogger.child({ moduleTag: `[${options.module}]` })
	}
	return baseLogger
}

export type Logger = ReturnType<typeof getLogger>
