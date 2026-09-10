import { createReadStream, createWriteStream, existsSync, mkdirSync, readdirSync, unlinkSync, type WriteStream } from "node:fs"
import { join } from "node:path"
import { createInterface } from "node:readline"
import type { LogSink } from "../Logger"
import { LOG_LEVEL_RANK, type LogRecordDto, type LogRecordLevel } from "./dto"

/**
 * Records held in memory. This is the live tail, not the history — the history
 * is the file. Sized so that a filler at `info` keeps its whole launch in
 * memory and never reads from disk to answer a query.
 */
export const DEFAULT_LOG_CAPACITY = 5000

/** Launch files kept on disk. Older ones are pruned when a new launch starts. */
export const DEFAULT_LAUNCHES_KEPT = 5

/**
 * Records one unbounded query returns. Deliberately not the ring capacity: the
 * history on disk is larger than memory, and defaulting to the ring would cap
 * every answer at the size of the cache in front of it.
 */
export const DEFAULT_QUERY_LIMIT = 2000

/** A serialized error with a stack fits comfortably; an accidental blob does not. */
const MAX_DETAIL_CHARS = 4000

const LEVEL_BY_VALUE: Record<number, LogRecordLevel> = {
	10: "trace",
	20: "debug",
	30: "info",
	40: "warn",
	50: "error",
	60: "fatal",
}

/**
 * pino writes its own numeric level first and then the caller's merge object
 * verbatim, so `logger.warn({ level: "debug" }, …)` emits *two* `level` keys and
 * `JSON.parse` keeps the caller's. Reading the numeric one off the line prefix
 * keeps that record instead of dropping it; the caller's value is put back into
 * `detail`, where payload belongs.
 */
const LEVEL_PREFIX = /^\s*\{\s*"level"\s*:\s*(\d+)/

/** Fields pino puts on every record: represented already, or noise for a dashboard. */
const ENVELOPE_FIELDS = new Set(["level", "time", "msg", "moduleTag", "pid", "hostname"])

const LAUNCH_FILE = /^simplex-[\dT-]+\.log$/

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

/** The read side the UI server needs; `LogStore` is the only implementation outside tests. */
export interface LogTail {
	recent(query: LogQuery): Promise<LogRecordDto[]>
	/** Calls `listener` for every record captured from now on; returns an unsubscribe. */
	subscribe(listener: (record: LogRecordDto) => void): () => void
	/** How the dashboard describes its own coverage. */
	stats(): { capacity: number; captured: number; persisted: boolean; path?: string }
}

export interface LogStoreOptions {
	capacity?: number
	/** Directory to write this launch's log into. Omitted, the store is memory-only. */
	dir?: string
	launchesKept?: number
	/** Injected by tests so a launch filename is deterministic. */
	startedAt?: Date
}

/**
 * Everything the filler has logged since launch, fed by registering {@link sink}
 * on a `LoggerContext` exactly as the console sink is.
 *
 * Two tiers, because "all of it" and "fast" pull in opposite directions. Every
 * record is appended to this launch's NDJSON file, which is the history and is
 * bounded only by the disk. The last {@link DEFAULT_LOG_CAPACITY} are also kept
 * in memory, which is what the live stream broadcasts and what answers a query
 * as long as nothing has been evicted — the common case for a filler at `info`.
 * Once the ring overflows, a query reads the older half back off the file.
 *
 * It still only sees what the context's level lets through: nothing here can
 * recover a debug record from a filler running at `info`. That is what the
 * dashboard's level control is for.
 */
export class LogStore implements LogTail {
	private readonly ring: Array<LogRecordDto | undefined>
	private head = 0
	private filled = false
	private nextSeq = 1
	/** Seq of the oldest record still in memory; everything below it lives only on disk. */
	private oldestInRing = 1
	private readonly listeners = new Set<(record: LogRecordDto) => void>()
	private stream?: WriteStream
	private file?: string

	readonly capacity: number

	constructor(options: LogStoreOptions = {}) {
		this.capacity = Math.max(1, Math.floor(options.capacity ?? DEFAULT_LOG_CAPACITY))
		this.ring = new Array(this.capacity)
		if (options.dir) this.openLaunchFile(options.dir, options.launchesKept, options.startedAt)
	}

	/**
	 * A destination to register with `addLogSink` or `SimplexOptions.logger`.
	 * Safe to register on several contexts at once — the process-wide one and a
	 * filler's own — since records are ordered by arrival, not by origin.
	 */
	sink(): LogSink {
		return { write: (line: string) => this.write(line) }
	}

	stats() {
		return {
			capacity: this.capacity,
			captured: this.nextSeq - 1,
			persisted: Boolean(this.stream),
			path: this.file,
		}
	}

	/** Ends this launch's file. The records stay on disk. */
	close(): void {
		this.stream?.end()
		this.stream = undefined
	}

	private write(line: string): void {
		// pino writes one record per call, but splitting costs nothing and means a
		// sink that batches lines cannot silently drop all but the first.
		for (const chunk of line.split("\n")) {
			if (chunk.trim().length === 0) continue
			const record = parseRecord(chunk, this.nextSeq)
			if (!record) continue
			this.nextSeq++
			this.append(record)
		}
	}

	private append(record: LogRecordDto): void {
		if (this.stream) {
			try {
				// Never awaited: a slow disk must not stall the fill that logged this.
				this.stream.write(`${JSON.stringify(record)}\n`)
			} catch {
				// A full or unwritable disk costs the history, not the filler.
			}
		}
		// O(1) eviction. An array + splice() shifts the whole buffer on every
		// record once full, which measured at ~7x the cost of everything else on
		// this path put together.
		if (this.filled) this.oldestInRing = (this.ring[this.head]?.seq ?? this.oldestInRing) + 1
		this.ring[this.head] = record
		this.head = (this.head + 1) % this.capacity
		if (this.head === 0) this.filled = true

		for (const listener of this.listeners) {
			try {
				listener(record)
			} catch {
				// A subscriber that throws is a dead browser tab, not a reason to fail a fill.
			}
		}
	}

	/** Oldest first. Reads the launch file only when the answer predates the ring. */
	async recent(query: LogQuery = {}): Promise<LogRecordDto[]> {
		const limit = query.limit ?? DEFAULT_QUERY_LIMIT
		// A seq from a previous launch — the counter restarts at 1 — would otherwise
		// filter out everything forever. Treat it as "no resume point".
		const stale = query.after !== undefined && query.after >= this.nextSeq
		const effective: LogQuery = { ...query, after: stale ? undefined : query.after }

		const fromRing = this.ringMatches(effective)
		// The ring already fills the page, so nothing older can survive the slice
		// below. Checked before `needsHistory` because the scan is the expensive
		// part: on a filler at debug the launch file is gigabytes, and the search
		// box would repeat it per debounced keystroke.
		if (fromRing.length >= limit) return fromRing.slice(-limit)
		const needsHistory = this.filled && (effective.after === undefined || effective.after < this.oldestInRing - 1)
		if (!needsHistory || !this.file) return fromRing.slice(-limit)

		// Everything below the ring's oldest seq lives only on the file. Splitting
		// at that boundary also sidesteps the write stream's buffer: records not
		// yet flushed to disk are exactly the ones the ring still holds.
		const fromFile = await this.scanFile(effective, this.oldestInRing, limit)
		return fromFile.concat(fromRing).slice(-limit)
	}

	subscribe(listener: (record: LogRecordDto) => void): () => void {
		this.listeners.add(listener)
		return () => {
			this.listeners.delete(listener)
		}
	}

	/** In-memory matches, oldest first. */
	private ringMatches(query: LogQuery): LogRecordDto[] {
		const out: LogRecordDto[] = []
		const count = this.filled ? this.capacity : this.head
		for (let i = 0; i < count; i++) {
			const record = this.ring[(this.head - count + i + this.capacity * 2) % this.capacity]
			if (record && matchesLogQuery(record, query)) out.push(record)
		}
		return out
	}

	/** Matches with `seq < before`, oldest first, keeping at most `limit`. */
	private async scanFile(query: LogQuery, before: number, limit: number): Promise<LogRecordDto[]> {
		const file = this.file
		if (!file || !existsSync(file)) return []
		const kept: LogRecordDto[] = []
		try {
			const lines = createInterface({ input: createReadStream(file, { encoding: "utf-8" }), crlfDelay: Infinity })
			for await (const line of lines) {
				if (line.length === 0) continue
				let record: LogRecordDto
				try {
					record = JSON.parse(line) as LogRecordDto
				} catch {
					continue
				}
				if (record.seq >= before) break
				if (!matchesLogQuery(record, query)) continue
				kept.push(record)
				// Bounded regardless of how large the launch file has grown.
				if (kept.length > limit) kept.shift()
			}
			lines.close()
		} catch {
			// An unreadable file costs history, not the request: the ring still answered.
		}
		return kept
	}

	/**
	 * Starts writing this launch's history to `dir`, pruning older launches.
	 *
	 * Separate from construction because the store is registered at module load —
	 * before `--data-dir` has been parsed — so that the very first records of a
	 * launch are captured. Whatever the ring already holds is written out first,
	 * which is what makes the file a complete record rather than one starting
	 * wherever the CLI got around to calling this.
	 */
	openLaunchFile(dir: string, launchesKept = DEFAULT_LAUNCHES_KEPT, startedAt?: Date): void {
		if (this.stream) return
		try {
			mkdirSync(dir, { recursive: true })
			const existing = readdirSync(dir)
				.filter((name) => LAUNCH_FILE.test(name))
				.sort()
			// Keep room for the launch about to start.
			for (const stale of existing.slice(0, Math.max(0, existing.length - launchesKept + 1))) {
				try {
					unlinkSync(join(dir, stale))
				} catch {
					// A file someone else holds open is not worth failing a boot over.
				}
			}
			// Milliseconds, not seconds: `RestartSec=0` restarts land inside the same
			// second, and an appended second run would put two seq-1..N sequences in
			// one file — which `scanFile` would read back as this launch's history,
			// colliding with the ring on seq. `wx` refuses a collision outright
			// rather than appending into someone else's launch.
			const stamp = (startedAt ?? new Date()).toISOString().replace(/[:.]/g, "-").replace(/Z$/, "")
			this.file = join(dir, `simplex-${stamp}.log`)
			this.stream = createWriteStream(this.file, { flags: "wx" })
			// Without a handler an EACCES/ENOSPC on the stream is an unhandled 'error' event.
			this.stream.on("error", () => {
				// A full disk, a permission change, or the `wx` collision above. The
				// path goes with the stream: a half-owned file must not be read back
				// as this launch's history.
				this.stream = undefined
				this.file = undefined
			})
			for (const record of this.ringMatches({})) this.stream.write(`${JSON.stringify(record)}\n`)
		} catch {
			// No writable data dir: the dashboard falls back to the in-memory tail.
			this.file = undefined
			this.stream = undefined
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
	if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return undefined

	// The prefix wins: a caller-supplied `level` in the payload displaces pino's
	// own in the parsed object, and dropping the record over that would lose the
	// very line that records a level change.
	const prefix = LEVEL_PREFIX.exec(line)
	const numeric = prefix ? Number(prefix[1]) : typeof parsed.level === "number" ? parsed.level : undefined
	const level = numeric === undefined ? undefined : LEVEL_BY_VALUE[numeric]
	if (!level) return undefined

	const detailFields: Record<string, unknown> = {}
	for (const [key, value] of Object.entries(parsed)) {
		if (!ENVELOPE_FIELDS.has(key)) detailFields[key] = value
	}
	// A payload `level` survived JSON.parse and displaced the envelope's; it is
	// data the operator logged, so show it rather than let the filter swallow it.
	if (parsed.level !== undefined && parsed.level !== numeric) detailFields.level = parsed.level

	let detail = Object.keys(detailFields).length > 0 ? JSON.stringify(detailFields) : undefined
	if (detail && detail.length > MAX_DETAIL_CHARS) {
		// `slice` alone yields a V8 SlicedString that pins the whole parent, so the
		// cap would bound what the page shows without bounding what the process
		// holds. A Buffer round-trip forces a flat copy and releases the original.
		detail = `${Buffer.from(detail.slice(0, MAX_DETAIL_CHARS), "utf-8").toString("utf-8")}…`
	}

	// `[filler]` -> `filler`: the brackets are the console format's, not part of the name.
	const moduleTag = typeof parsed.moduleTag === "string" ? parsed.moduleTag.replace(/^\[|\]$/g, "") : undefined

	return {
		seq,
		time: typeof parsed.time === "number" ? parsed.time : Date.now(),
		level,
		module: moduleTag || undefined,
		msg: typeof parsed.msg === "string" ? parsed.msg : "",
		detail,
	}
}
