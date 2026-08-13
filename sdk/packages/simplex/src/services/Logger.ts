import pino, { stdSerializers } from "pino"

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error"

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
					error: stdSerializers.err,
					err: stdSerializers.err,
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
