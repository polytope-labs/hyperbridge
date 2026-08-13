import pino, { stdSerializers } from "pino"
export type LogLevel = "trace" | "debug" | "info" | "warn" | "error"

type LoggerOptions = {
	module?: string
}

/**
 * viem hangs the whole contract ABI off its errors and repeats the same message
 * once per wrapper in the cause chain. Serialised verbatim, one failed
 * `readContract` prints several hundred lines of ABI JSON. None of it says
 * anything the short message and the root cause don't.
 */
const NOISY_ERROR_FIELDS = [
	"abi",
	"args",
	"formattedArgs",
	"metaMessages",
	"docsPath",
	"version",
	"contractAddress",
	"functionName",
	"sender",
] as const

/** Deep frames are bundler offsets into dist/bin/simplex.js — useless for diagnosis. */
const MAX_STACK_FRAMES = 6

/** `at fn (/very/long/.pnpm/viem@x/node_modules/viem/a/b.ts:1:2)` -> `at fn (b.ts:1:2)`. */
function compactFrame(frame: string): string {
	return frame.trim().replace(/\(?([^\s(]*[/\\])?([^/\\\s()]+:\d+:\d+)\)?$/, "($2)")
}

/** Innermost `cause` message: the line that actually says what went wrong. */
function rootCause(error: unknown): string | undefined {
	let current: unknown = error
	let deepest: string | undefined
	for (let depth = 0; depth < 8; depth++) {
		const cause = (current as { cause?: unknown })?.cause
		if (!cause) break
		if (cause instanceof Error) {
			deepest = `${cause.name}: ${cause.message}`.split("\n")[0]
		} else if (typeof cause === "string") {
			deepest = cause.split("\n")[0]
		}
		current = cause
	}
	return deepest
}

/** The endpoint viem was talking to, recovered before metaMessages is dropped. */
function endpointOf(error: unknown): string | undefined {
	const meta = (error as { metaMessages?: unknown })?.metaMessages
	if (!Array.isArray(meta)) return undefined
	const line = meta.find((m) => typeof m === "string" && m.startsWith("URL: "))
	return typeof line === "string" ? line.slice(5) : undefined
}

/**
 * Keeps what identifies the failure — type, short message, details, the failing
 * endpoint and the root cause — and discards the ABI dump, the repeated message
 * bodies and the deep bundler stack frames.
 */
function compactError(error: unknown): unknown {
	if (!(error instanceof Error)) return stdSerializers.err(error as Error)

	const serialized = stdSerializers.err(error) as Record<string, unknown>
	const endpoint = endpointOf(error)
	const cause = rootCause(error)

	for (const field of NOISY_ERROR_FIELDS) delete serialized[field]

	// `message` restates shortMessage plus every wrapper's context; the short
	// form carries the same information in one line.
	const shortMessage = serialized.shortMessage
	if (typeof shortMessage === "string" && shortMessage.length > 0) {
		serialized.message = shortMessage
		delete serialized.shortMessage
	} else if (typeof serialized.message === "string") {
		serialized.message = serialized.message.split("\n")[0]
	}

	if (typeof serialized.stack === "string") {
		const frames = serialized.stack
			.split("\n")
			.filter((line) => line.trim().startsWith("at "))
			.slice(0, MAX_STACK_FRAMES)
			.map(compactFrame)
		if (frames.length > 0) serialized.stack = frames.join(" <- ")
		else delete serialized.stack
	}

	// `name` is always the same string as `type`.
	delete serialized.name

	if (endpoint) serialized.endpoint = endpoint
	if (cause && cause !== serialized.message) serialized.cause = cause

	return serialized
}

let logLevel: LogLevel = "info"

let baseLogger: pino.Logger

// Initialize the logger with current config
function initializeLogger() {
	baseLogger = pino({
		level: logLevel,
		serializers: {
			error: compactError,
			err: compactError,
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
