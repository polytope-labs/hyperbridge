import pino, { stdSerializers } from "pino"

export const LOG_LEVELS = ["trace", "debug", "info", "warn", "error"] as const

export type LogLevel = (typeof LOG_LEVELS)[number]

/**
 * Anything that accepts a line of text: `process.stdout`, a `fs` write stream, a
 * socket, a `pino-pretty` transform, or an object literal that forwards into
 * your own logger. Node's `Writable` satisfies it as-is.
 */
export interface LogSink {
	write(line: string): void
}

type LogFn = {
	(obj: unknown, msg?: string, ...args: unknown[]): void
	(msg: string, ...args: unknown[]): void
}

export interface Logger {
	trace: LogFn
	debug: LogFn
	info: LogFn
	warn: LogFn
	error: LogFn
	fatal: LogFn
}

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
	// form carries the same information in one line. This applies to viem's
	// errors, which is where the duplication comes from — an error we built
	// ourselves has no wrapper padding to strip, and truncating it would throw
	// away the per-provider failure list that is the whole point of the message.
	const shortMessage = serialized.shortMessage
	if (error.name === "QuorumError") {
		// Keep it whole.
	} else if (typeof shortMessage === "string" && shortMessage.length > 0) {
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

/**
 * An isolated logging destination: its own sinks, its own level, its own pino
 * instance.
 *
 * Each `Simplex` owns one, so two fillers in a process log to their own sinks at
 * their own verbosity and never cross-contaminate. Services receive the context
 * their filler was started with and resolve module loggers from it.
 */
export class LoggerContext {
	private level: LogLevel
	private readonly sinks = new Set<LogSink>()
	private base: pino.Logger | undefined
	private readonly children = new Map<string, pino.Logger>()

	constructor(options: { level?: LogLevel; sink?: LogSink } = {}) {
		// Checked here rather than at the first log: pino throws on an unknown level,
		// and with a silent default that throw would surface much later, from
		// whatever happened to log first after a sink attached.
		if (options.level && !LOG_LEVELS.includes(options.level)) {
			throw new Error(`Unknown log level '${options.level}'. Use one of: ${LOG_LEVELS.join(", ")}`)
		}
		this.level = options.level ?? "info"
		if (options.sink) this.sinks.add(options.sink)
	}

	/**
	 * Drops the memoized loggers so the next call rebuilds at the current level
	 * and sink set. Module loggers are wrappers rather than pino instances
	 * precisely so this reaches loggers that already exist — replacing the base
	 * logger would leave every service that had captured a child logging at the
	 * old level forever.
	 */
	private invalidate(): void {
		this.base = undefined
		this.children.clear()
	}

	private baseLogger(): pino.Logger {
		if (this.base) return this.base
		const sinks = this.sinks
		this.base = pino(
			{
				// With no sink registered nothing can consume a record, so don't pay to
				// serialize one. This is what makes an unconfigured filler free as well
				// as silent.
				level: sinks.size === 0 ? "silent" : this.level,
				serializers: {
					error: compactError,
					err: compactError,
				},
			},
			{
				write(line: string) {
					for (const sink of sinks) {
						try {
							sink.write(line)
						} catch {
							// A broken sink is the host's problem, not a reason to fail a fill.
						}
					}
				},
			},
		)
		return this.base
	}

	private childFor(module?: string): pino.Logger {
		if (!module) return this.baseLogger()
		let child = this.children.get(module)
		if (!child) {
			child = this.baseLogger().child({ moduleTag: `[${module}]` })
			this.children.set(module, child)
		}
		return child
	}

	/** Registers a destination and returns a function that removes it. */
	addSink(sink: LogSink): () => void {
		this.sinks.add(sink)
		this.invalidate()
		return () => {
			this.sinks.delete(sink)
			this.invalidate()
		}
	}

	/** Sets verbosity, including for loggers already handed out. */
	setLevel(level: LogLevel): void {
		if (!LOG_LEVELS.includes(level)) {
			throw new Error(`Unknown log level '${level}'. Use one of: ${LOG_LEVELS.join(", ")}`)
		}
		this.level = level
		this.invalidate()
	}

	/** A logger tagged with `module`. Safe to hold; it resolves the current state per call. */
	get(module?: string): Logger {
		return new ModuleLogger(this, module)
	}

	/** @internal — used by {@link ModuleLogger} to resolve per call. */
	resolve(module?: string): pino.Logger {
		return this.childFor(module)
	}
}

/** Resolves through its context per call, so level and sink changes apply immediately. */
class ModuleLogger implements Logger {
	constructor(
		private readonly context: LoggerContext,
		private readonly module?: string,
	) {}

	private forward(method: keyof Logger): LogFn {
		// biome-ignore lint/suspicious/noExplicitAny: pino's LogFn overloads don't survive a generic forward
		return (...args: any[]) => (this.context.resolve(this.module)[method] as (...a: any[]) => void)(...args)
	}

	trace = this.forward("trace")
	debug = this.forward("debug")
	info = this.forward("info")
	warn = this.forward("warn")
	error = this.forward("error")
	fatal = this.forward("fatal")
}

/**
 * The fallback context, used by code that is not scoped to a single filler: the
 * CLI, the setup wizard, and anything constructed before a `Simplex` exists.
 *
 * An embedded filler never logs here — `Simplex` creates its own context — so
 * leaving this unconfigured is what keeps the library silent in a host process.
 */
const processContext = new LoggerContext()

/**
 * Registers a destination on the process-wide context and returns a function
 * that removes it.
 *
 * This is for the application layer. To capture a specific filler's output, pass
 * `SimplexOptions.logger` instead — a filler's records go to its own context,
 * not here.
 */
export function addLogSink(sink: LogSink): () => void {
	return processContext.addSink(sink)
}

/** Sets the level of the process-wide context. Does not affect running fillers. */
export function configureLogger(level: LogLevel): void {
	processContext.setLevel(level)
}

/** A logger on the process-wide context. Instance-scoped code resolves from its own {@link LoggerContext}. */
export function getLogger(moduleOrOptions?: string | LoggerOptions): Logger {
	const options: LoggerOptions =
		typeof moduleOrOptions === "string" ? { module: moduleOrOptions } : moduleOrOptions || {}
	return processContext.get(options.module)
}

/** The fallback context, for constructors that were given none. */
export function defaultLoggerContext(): LoggerContext {
	return processContext
}

/**
 * Resolves a module logger from a context that may be absent.
 *
 * Services read their context off the `FillerConfigService` or
 * `ChainClientManager` they were constructed with, and `bootFiller` always
 * supplies real ones. Hand-built test doubles generally do not, so this falls
 * back to the process context rather than forcing every mock to carry a logger.
 */
export function moduleLogger(context: LoggerContext | undefined, module: string): Logger {
	return (context ?? processContext).get(module)
}
