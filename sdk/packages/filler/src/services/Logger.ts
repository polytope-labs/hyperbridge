import pino from "pino"
import { LoggingConfig } from "./FillerConfigService"

type LoggerOptions = {
	module?: string
}

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error"

let logLevel: LogLevel = "info"

let baseLogger: pino.Logger

// Initialize the logger with current config
function initializeLogger() {
	baseLogger = pino({
		level: logLevel,
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

export function configureLogger(config: LoggingConfig) {
	if (config.level) {
		logLevel = config.level
		initializeLogger()
	}
}

export function getLogger(moduleOrOptions?: string | LoggerOptions) {
	console.log("Log level:", logLevel)
	const options: LoggerOptions =
		typeof moduleOrOptions === "string" ? { module: moduleOrOptions } : moduleOrOptions || {}
	if (options.module) {
		return baseLogger.child({ moduleTag: `[${options.module}]` })
	}
	return baseLogger
}

export type Logger = ReturnType<typeof getLogger>
