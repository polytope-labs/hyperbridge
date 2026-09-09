import type { LogSink } from "../Logger"
import { LOG_LEVEL_RANK, type LogRecordDto, type LogRecordLevel } from "./dto"

/**
 * How many records the dashboard can look back over. Roughly a megabyte at the
 * record sizes this filler produces, which buys a few hours at `info` and a few
 * minutes at `trace` — long enough to answer "what happened just now", which is
 * what the page is for. The console sink is still where a full history lives.
 */
export const DEFAULT_LOG_BUFFER_CAPACITY = 2000

/** A serialized error with a stack fits comfortably; an accidental blob does not get to sit in memory. */
const MAX_DETAIL_CHARS = 4000

const LEVEL_BY_VALUE: Record<number, LogRecordLevel> = {
	10: "trace",
	20: "debug",
	30: "info",
	40: "warn",
	50: "error",
	60: "fatal",
}

/** Fields pino puts on every record; they are either represented already or noise for a local dashboard. */
const ENVELOPE_FIELDS = new Set(["level", "time", "msg", "moduleTag", "pid", "hostname"])

/** What the dashboard filters by. Absent fields match everything. */
export interface LogQuery {
	/** Minimum level, inclusive: "warn" matches warn, error and fatal. */
	level?: LogRecordLevel
	/** Case-insensitive substring, matched against the message, module and detail. */
	q?: string
	/** Only records newer than this seq — how a reconnecting stream resumes without gaps or repeats. */
	after?: number
	/** Most recent N of whatever matched. */
	limit?: number
}

/** True when `record` satisfies every field the query sets. */
export function matchesLogQuery(record: LogRecordDto, query: LogQuery): boolean {
	if (query.level && LOG_LEVEL_RANK[record.level] < LOG_LEVEL_RANK[query.level]) return false
	if (query.after !== undefined && record.seq <= query.after) return false
	if (query.q) {
		const needle = query.q.toLowerCase()
		const haystack = `${record.msg} ${record.module ?? ""} ${record.detail ?? ""}`.toLowerCase()
		if (!haystack.includes(needle)) return false
	}
	return true
}

/** The read side the UI server needs; `LogBuffer` is the only implementation outside tests. */
export interface LogTail {
	recent(query: LogQuery): LogRecordDto[]
	/** Calls `listener` for every record captured from now on; returns an unsubscribe. */
	subscribe(listener: (record: LogRecordDto) => void): () => void
	capacity: number
}

/**
 * A bounded, in-memory tail of the log stream, fed by registering {@link sink}
 * on a `LoggerContext` exactly like the console sink is.
 *
 * It sees only what the context's level lets through — nothing here can recover
 * a debug record from a filler running at `info`; the operator has to raise the
 * capture level first, which is what `PUT /api/log-level` is for.
 */
export class LogBuffer implements LogTail {
	/** Oldest first. Trimmed from the front once it exceeds `capacity`. */
	private records: LogRecordDto[] = []
	private nextSeq = 1
	private readonly listeners = new Set<(record: LogRecordDto) => void>()

	constructor(readonly capacity: number = DEFAULT_LOG_BUFFER_CAPACITY) {}

	/**
	 * A destination to register with `addLogSink` or `Simplex.addLogSink`. Safe
	 * to register on several contexts at once — the process-wide one and a
	 * filler's own — since records are ordered by arrival, not by origin.
	 */
	sink(): LogSink {
		return { write: (line: string) => this.write(line) }
	}

	private write(line: string): void {
		// pino writes one record per call, but splitting costs nothing and means a
		// sink that batches lines cannot silently drop all but the first.
		for (const chunk of line.split("\n")) {
			if (chunk.trim().length === 0) continue
			const record = parseRecord(chunk, this.nextSeq)
			if (!record) continue
			this.nextSeq++
			this.records.push(record)
			if (this.records.length > this.capacity) this.records.splice(0, this.records.length - this.capacity)
			for (const listener of this.listeners) {
				try {
					listener(record)
				} catch {
					// A subscriber that throws is a dead browser tab, not a reason to fail a fill.
				}
			}
		}
	}

	recent(query: LogQuery = {}): LogRecordDto[] {
		const matched = this.records.filter((record) => matchesLogQuery(record, query))
		return query.limit !== undefined && matched.length > query.limit ? matched.slice(-query.limit) : matched
	}

	subscribe(listener: (record: LogRecordDto) => void): () => void {
		this.listeners.add(listener)
		return () => {
			this.listeners.delete(listener)
		}
	}
}

/** NDJSON from pino to a display record; undefined for anything that is not one. */
function parseRecord(line: string, seq: number): LogRecordDto | undefined {
	let parsed: Record<string, unknown>
	try {
		parsed = JSON.parse(line) as Record<string, unknown>
	} catch {
		return undefined
	}
	if (typeof parsed !== "object" || parsed === null) return undefined
	const level = LEVEL_BY_VALUE[parsed.level as number]
	if (!level) return undefined

	const detailFields: Record<string, unknown> = {}
	for (const [key, value] of Object.entries(parsed)) {
		if (!ENVELOPE_FIELDS.has(key)) detailFields[key] = value
	}
	const detail = Object.keys(detailFields).length > 0 ? JSON.stringify(detailFields) : undefined

	// `[filler]` -> `filler`: the brackets are the console format's, not part of the name.
	const moduleTag = typeof parsed.moduleTag === "string" ? parsed.moduleTag.replace(/^\[|\]$/g, "") : undefined

	return {
		seq,
		time: typeof parsed.time === "number" ? parsed.time : Date.now(),
		level,
		module: moduleTag || undefined,
		msg: typeof parsed.msg === "string" ? parsed.msg : "",
		detail: detail && detail.length > MAX_DETAIL_CHARS ? `${detail.slice(0, MAX_DETAIL_CHARS)}…` : detail,
	}
}
