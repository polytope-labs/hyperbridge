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

let logLevel: LogLevel = "info"
const sinks = new Set<LogSink>()

let base: pino.Logger | undefined
const children = new Map<string, pino.Logger>()

/**
 * Drops the memoized loggers so the next call rebuilds at the current level and
 * sink set. Callers hold `ModuleLogger` wrappers rather than pino instances
 * precisely so this takes effect on loggers that already exist — the previous
 * implementation replaced the base logger, which left every service that had
 * already captured a child logging at the old level forever.
 */
function invalidate(): void {
	base = undefined
	children.clear()
}

function baseLogger(): pino.Logger {
	if (base) return base
	base = pino(
		{
			// With no sink registered nothing can consume a record, so don't pay to
			// serialize one. This is what makes an unconfigured library free as well
			// as silent.
			level: sinks.size === 0 ? "silent" : logLevel,
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
	return base
}

function childFor(module?: string): pino.Logger {
	if (!module) return baseLogger()
	let child = children.get(module)
	if (!child) {
		child = baseLogger().child({ moduleTag: `[${module}]` })
		children.set(module, child)
	}
	return child
}

/** Resolves the current logger per call, so level and sink changes apply immediately. */
class ModuleLogger implements Logger {
	constructor(private readonly module?: string) {}

	private forward(method: keyof Logger): LogFn {
		// biome-ignore lint/suspicious/noExplicitAny: pino's LogFn overloads don't survive a generic forward
		return (...args: any[]) => (childFor(this.module)[method] as (...a: any[]) => void)(...args)
	}

	trace = this.forward("trace")
	debug = this.forward("debug")
	info = this.forward("info")
	warn = this.forward("warn")
	error = this.forward("error")
	fatal = this.forward("fatal")
}

/**
 * Registers a destination for log records, one NDJSON line per record, and
 * returns a function that removes it.
 *
 * **Nothing is logged until a sink is registered.** A library has no business
 * writing to its host's stdout uninvited, so the default is silence — the CLI
 * opts into pretty-printed stdout, and an embedded filler logs wherever
 * `SimplexOptions.logger` points.
 *
 * Sinks are process-wide, not per-filler: two `Simplex` instances in one process
 * both write to every registered sink. Records carry a `moduleTag` but not an
 * instance id, so tag your sinks if you need to tell two fillers apart.
 */
export function addLogSink(sink: LogSink): () => void {
	sinks.add(sink)
	invalidate()
	return () => {
		sinks.delete(sink)
		invalidate()
	}
}

/**
 * Sets the verbosity of every logger, including ones already handed out. Level
 * alone produces no output — a sink is still required.
 */
export function configureLogger(level: LogLevel): void {
	logLevel = level
	invalidate()
}

export function getLogger(moduleOrOptions?: string | LoggerOptions): Logger {
	const options: LoggerOptions =
		typeof moduleOrOptions === "string" ? { module: moduleOrOptions } : moduleOrOptions || {}
	return new ModuleLogger(options.module)
}
