import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"
import { LOG_LEVELS, LOG_LEVEL_RANK } from "@/services/server/dto"
import { api } from "../api"
import { CloseIcon } from "../components/InterfaceIcons"
import { PillTabs } from "../components/PillTabs"
import type { LogRecordDto, LogRecordLevel, LogsDto } from "../types"

/**
 * Rows kept in the DOM. The server buffer is the same order of magnitude, so
 * this is not what limits the history — it stops a long trace session from
 * growing the page without bound.
 */
const MAX_ROWS = 2000

/** Typing re-runs the query on the server; a pause this long makes that one request, not one per key. */
const SEARCH_DEBOUNCE_MS = 250

/** New records are applied in batches — at trace level they arrive far faster than React should render. */
const FLUSH_MS = 150

/** How close to the bottom still counts as "following the tail". */
const STICKY_SLACK_PX = 40

/** Wait before re-opening a feed that dropped; the filler may be restarting. */
const RETRY_MS = 2000

/** `fatal` is a level a record can carry but not one an operator selects. */
type SelectableLevel = (typeof LOG_LEVELS)[number]

const LEVEL_OPTIONS = LOG_LEVELS.map((level) => ({ value: level, label: level.toUpperCase() }))

/** The filler's level as read back from its config, which is free-form text on disk. */
function captureLevelOf(value: string): LogRecordLevel {
	return value in LOG_LEVEL_RANK ? (value as LogRecordLevel) : "info"
}

/** "14:22:07.481" — logs are read against each other, so the seconds and millis are the point. */
function formatLogTime(ts: number): string {
	const at = new Date(ts)
	const pad = (value: number, width = 2) => String(value).padStart(width, "0")
	return `${pad(at.getHours())}:${pad(at.getMinutes())}:${pad(at.getSeconds())}.${pad(at.getMilliseconds(), 3)}`
}

/** The search term marked up wherever it occurs, so a hit is visible without re-reading the line. */
function Highlight(props: { text: string; term: string }) {
	const { text, term } = props
	const parts = useMemo(() => {
		if (!term) return [text]
		const needle = term.toLowerCase()
		const haystack = text.toLowerCase()
		const out: string[] = []
		let cursor = 0
		for (let at = haystack.indexOf(needle); at !== -1; at = haystack.indexOf(needle, cursor)) {
			out.push(text.slice(cursor, at), text.slice(at, at + needle.length))
			cursor = at + needle.length
		}
		out.push(text.slice(cursor))
		return out
	}, [text, term])
	if (parts.length === 1) return <>{parts[0]}</>
	// Odd indices are the matches: the split above alternates gap, hit, gap, hit…
	// The index is the key because it *is* the identity here — position in one
	// string, re-derived from scratch whenever that string or the term changes.
	return (
		<>
			{parts.map((part, index) =>
				index % 2 === 1 ? <mark key={index}>{part}</mark> : <span key={index}>{part}</span>,
			)}
		</>
	)
}


/**
 * A record's fields, rendered as tokens rather than as the raw JSON string.
 *
 * `detail` is a JSON object serialized by the store — but not always a *valid*
 * one: an oversized record is truncated mid-string and gets an ellipsis, so
 * every path here has to survive `JSON.parse` throwing.
 */
type DetailToken = { text: string; kind: "key" | "string" | "number" | "literal" | "punct" }

/** Tokens for one value. `indent` >= 0 pretty-prints; -1 keeps it on one line. */
function walkValue(value: unknown, out: DetailToken[], indent: number): void {
	const push = (text: string, kind: DetailToken["kind"]): void => {
		out.push({ text, kind })
	}
	const pretty = indent >= 0
	const pad = (depth: number) => (pretty ? "\n" + "  ".repeat(depth) : "")

	if (value === null) return push("null", "literal")
	if (typeof value === "boolean") return push(String(value), "literal")
	if (typeof value === "number") return push(String(value), "number")
	// Round-tripped rather than raw, so quotes and escapes render as they were.
	if (typeof value === "string") return push(JSON.stringify(value), "string")

	if (Array.isArray(value)) {
		if (value.length === 0) return push("[]", "punct")
		push("[", "punct")
		value.forEach((item, i) => {
			if (i) push(",", "punct")
			if (pretty) push(pad(indent + 1), "punct")
			else if (i) push(" ", "punct")
			walkValue(item, out, pretty ? indent + 1 : -1)
		})
		if (pretty) push(pad(indent), "punct")
		return push("]", "punct")
	}

	const entries = Object.entries(value as Record<string, unknown>)
	if (entries.length === 0) return push("{}", "punct")
	push("{", "punct")
	entries.forEach(([key, val], i) => {
		if (i) push(",", "punct")
		if (pretty) push(pad(indent + 1), "punct")
		else push(" ", "punct")
		push(key, "key")
		push(": ", "punct")
		walkValue(val, out, pretty ? indent + 1 : -1)
	})
	if (pretty) push(pad(indent), "punct")
	push(pretty ? "}" : " }", "punct")
}

/** Top-level fields without the outer braces — they are noise on every record. */
function tokenizeDetail(detail: string, pretty: boolean): DetailToken[] | null {
	let parsed: unknown
	try {
		parsed = JSON.parse(detail)
	} catch {
		return null
	}
	if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return null
	const out: DetailToken[] = []
	Object.entries(parsed as Record<string, unknown>).forEach(([key, value], i) => {
		if (i) out.push({ text: pretty ? "\n" : ", ", kind: "punct" })
		out.push({ text: key, kind: "key" })
		out.push({ text: ": ", kind: "punct" })
		walkValue(value, out, pretty ? 0 : -1)
	})
	return out
}

function DetailFields(props: { detail: string; term: string; pretty: boolean }) {
	const { detail, term, pretty } = props
	const tokens = useMemo(() => tokenizeDetail(detail, pretty), [detail, pretty])
	// A search hit that lands across a token boundary would render with nothing
	// marked — the same "highlighted row with no highlight" the clamp used to
	// cause. Fall back to the raw string so the match is always visible.
	const splitsMatch =
		Boolean(term) &&
		tokens !== null &&
		!tokens.some((t) => t.text.toLowerCase().includes(term.toLowerCase()))
	if (tokens === null || splitsMatch) return <Highlight text={detail} term={term} />
	return (
		<>
			{/* The index is the identity: a position in one serialized record, re-derived whenever it changes. */}
			{tokens.map((token, index) => (
				<span key={index} className={`jt-${token.kind}`}>
					<Highlight text={token.text} term={term} />
				</span>
			))}
		</>
	)
}

const LogRow = memo(function LogRow(props: { record: LogRecordDto; term: string }) {
	const { record, term } = props
	const [expanded, setExpanded] = useState(false)
	// A row the search matched on a field has to show that field. Clamping a hit
	// is how you get a highlighted row with nothing highlighted on it.
	const matchedInDetail = Boolean(term) && Boolean(record.detail?.toLowerCase().includes(term.toLowerCase()))
	return (
		<li className="log-row" data-level={record.level}>
			<time dateTime={new Date(record.time).toISOString()}>{formatLogTime(record.time)}</time>
			<span className="log-level">{record.level}</span>
			<span className="log-module">{record.module ? <Highlight text={record.module} term={term} /> : null}</span>
			<span className="log-body" data-expanded={expanded || matchedInDetail || undefined}>
				<span className="log-msg">
					<Highlight text={record.msg} term={term} />
				</span>
				{/* A real separator, not just the margin: a copied row has to read the
				    way it looks, and these get pasted into tickets. */}
				{record.detail ? "  " : null}
				{/* A span, not a button: a button is an atomic inline box that cannot
				    break across lines, so a long record's fields could not share the
				    message's line and the whole box dropped below it. */}
				{record.detail ? (
					<span
						className="log-detail"
						role="button"
						tabIndex={0}
						onClick={() => setExpanded((open) => !open)}
						onKeyDown={(event) => {
							if (event.key !== "Enter" && event.key !== " ") return
							event.preventDefault()
							setExpanded((open) => !open)
						}}
						title={expanded ? "Collapse this record" : "Show the whole record"}
					>
						<DetailFields detail={record.detail} term={term} pretty={expanded} />
					</span>
				) : null}
			</span>
		</li>
	)
})

/**
 * The filler's log, from launch: one feed, filtered by level and by a search
 * over the message, the module and the serialized fields.
 *
 * The level pills set the filler's own log level, in both directions — see
 * {@link Logs.chooseLevel}. Everything captured is on disk, so the page can
 * search back past what memory holds.
 */
export function Logs() {
	const [level, setLevel] = useState<SelectableLevel>("info")
	const [search, setSearch] = useState("")
	const [term, setTerm] = useState("")
	const [records, setRecords] = useState<LogRecordDto[]>([])
	/** Records at or below this seq are hidden: what Clear leaves behind, so a re-query does not undo it. */
	const [floor, setFloor] = useState(0)
	const [coverage, setCoverage] = useState<{ captured: number; persisted: boolean; path?: string }>()
	const [applying, setApplying] = useState<SelectableLevel>()
	const [paused, setPaused] = useState(false)
	const [live, setLive] = useState(false)
	const [error, setError] = useState<string>()
	/** Bumped to re-run the feed effect after a drop. */
	const [attempt, setAttempt] = useState(0)

	/** Cleared once the filler's own level has been adopted, so it happens exactly once. */
	const unsynced = useRef(true)
	const scroller = useRef<HTMLDivElement | null>(null)
	const sticky = useRef(true)
	const pending = useRef<LogRecordDto[]>([])
	const flushTimer = useRef<number | undefined>(undefined)

	useEffect(() => {
		const timer = window.setTimeout(() => setTerm(search.trim()), SEARCH_DEBOUNCE_MS)
		return () => window.clearTimeout(timer)
	}, [search])

	// One effect owns the whole feed: the backfill request and the stream that
	// resumes from its last seq. Any filter change tears both down and starts
	// again, which is also how Pause stops and Resume catches up.
	useEffect(() => {
		if (paused) return
		let cancelled = false
		let source: EventSource | undefined
		let retry: number | undefined
		const query = new URLSearchParams({ level })
		if (term) query.set("q", term)
		if (floor) query.set("after", String(floor))

		// A dropped feed is re-established by re-running this effect, not by
		// EventSource's own reconnect: the browser would re-open the same URL and
		// be handed the same replay again, duplicating every row already on the
		// page. Starting over re-reads the backfill and resumes from where the
		// page actually is.
		const scheduleRetry = () => {
			if (cancelled || retry !== undefined) return
			retry = window.setTimeout(() => setAttempt((n) => n + 1), RETRY_MS)
		}

		const flush = () => {
			flushTimer.current = undefined
			const batch = pending.current
			pending.current = []
			if (batch.length > 0) setRecords((prev) => [...prev, ...batch].slice(-MAX_ROWS))
		}

		void (async () => {
			try {
				const dto = await api.get<LogsDto>(`/api/logs?${query}&limit=${MAX_ROWS}`)
				if (cancelled) return
				setCoverage({ captured: dto.captured, persisted: dto.persisted, path: dto.path })
				// The pills show the filler's level, so on first load they have to
				// adopt it rather than assert a default the filler may not be at.
				if (unsynced.current) {
					unsynced.current = false
					const actual = captureLevelOf(dto.level)
					if (actual !== level && actual !== "fatal") setLevel(actual)
				}
				// `floor` is a seq from whichever process produced it, and the counter
				// restarts at 1. Once the filler has logged fewer records than the
				// mark Clear left behind, that mark belongs to a dead process.
				if (floor > dto.captured) setFloor(0)
				setRecords(dto.records)
				// A wholesale replacement is a fresh view; re-anchor to the tail so a
				// scroll made before a filter change does not freeze the feed.
				sticky.current = true
				setError(undefined)
				// Whatever was logged while that request was in flight is still in
				// the buffer; the stream replays it rather than skipping to now.
				query.set("after", String(dto.records[dto.records.length - 1]?.seq ?? floor))
				source = new EventSource(`/api/logs/stream?${query}`)
				source.onopen = () => setLive(true)
				// The server dropped frames to a reader that had stopped reading. The
				// records are still on disk, so start the feed over rather than splice
				// two non-adjacent stretches together.
				source.addEventListener("gap", () => setAttempt((n) => n + 1))
				source.onerror = () => {
					setLive(false)
					source?.close()
					scheduleRetry()
				}
				source.onmessage = (event) => {
					pending.current.push(JSON.parse(event.data) as LogRecordDto)
					if (flushTimer.current === undefined) flushTimer.current = window.setTimeout(flush, FLUSH_MS)
				}
			} catch (err) {
				if (cancelled) return
				setLive(false)
				setError(err instanceof Error ? err.message : String(err))
				scheduleRetry()
			}
		})()

		return () => {
			cancelled = true
			source?.close()
			window.clearTimeout(retry)
			window.clearTimeout(flushTimer.current)
			flushTimer.current = undefined
			pending.current = []
			setLive(false)
		}
	}, [level, term, floor, paused, attempt])

	// Follow the tail unless the operator has scrolled up to read something.
	useLayoutEffect(() => {
		if (!sticky.current) return
		const node = scroller.current
		if (node) node.scrollTop = node.scrollHeight
	}, [records])

	const onScroll = useCallback(() => {
		const node = scroller.current
		if (!node) return
		sticky.current = node.scrollHeight - node.scrollTop - node.clientHeight <= STICKY_SLACK_PX
	}, [])

	const clear = () => {
		sticky.current = true
		setFloor(records[records.length - 1]?.seq ?? floor)
		setRecords([])
	}

	/**
	 * The pills are the filler's log level, not a view filter over one — raising
	 * starts recording more, lowering stops recording it at all, which is how an
	 * operator keeps a long-running filler's log file from growing without bound.
	 *
	 * Awaited rather than optimistic: changing `level` re-runs the feed effect,
	 * and its `GET /api/logs` reports the level straight out of the config. Fired
	 * concurrently, that read can answer before the write lands and show the old
	 * level back to the operator who just changed it.
	 */
	const chooseLevel = async (next: SelectableLevel) => {
		if (next === level) return
		setApplying(next)
		try {
			await api.put("/api/log-level", { level: next })
			setLevel(next)
			setError(undefined)
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setApplying(undefined)
		}
	}

	// seq counts from 1, so the newest one the page holds *is* the number captured
	// — fresher than the count the last backfill reported.
	const captured = Math.max(coverage?.captured ?? 0, records[records.length - 1]?.seq ?? 0)

	return (
		<div className="operator-page-content operator-logs">
			<div className="log-toolbar">
				<PillTabs
					className="log-levels"
					options={LEVEL_OPTIONS}
					value={applying ?? level}
					onChange={chooseLevel}
					ariaLabel="Filler log level"
				/>
				<div className="log-search">
					<input
						type="search"
						value={search}
						placeholder="Search messages, modules and fields"
						aria-label="Search logs"
						onChange={(event) => setSearch(event.target.value)}
					/>
					{search ? (
						<button type="button" onClick={() => setSearch("")} aria-label="Clear search">
							<CloseIcon aria-hidden="true" />
						</button>
					) : null}
				</div>
				<div className="log-actions">
					<span className={`badge ${paused ? "warn" : live ? "ok" : "warn"}`}>
						{paused ? "paused" : live ? "live" : "reconnecting…"}
					</span>
					<button type="button" onClick={() => setPaused((value) => !value)}>
						{paused ? "Resume" : "Pause"}
					</button>
					<button type="button" onClick={clear} disabled={records.length === 0}>
						Clear
					</button>
				</div>
			</div>

			<div
				className="log-stream"
				ref={scroller}
				onScroll={onScroll}
				tabIndex={0}
				role="log"
				aria-label="Filler logs"
			>
				{records.length === 0 ? (
					<p className="operator-empty">
						{error
							? error
							: term
								? `Nothing matching “${term}” in the ${level} and above records kept since launch.`
								: "No log lines yet. New ones appear as the filler works."}
					</p>
				) : (
					<ul>
						{records.map((record) => (
							<LogRow key={record.seq} record={record} term={term} />
						))}
					</ul>
				)}
			</div>

			<p className="log-footer">
				<span>
					{records.length} line{records.length === 1 ? "" : "s"} shown
					{coverage ? ` · ${captured.toLocaleString()} captured since launch` : ""}
					{" · "}
					<span className="log-capture" title="Saved to the filler's config, so it survives a restart">
						recording <strong>{level}</strong> and above
					</span>
				</span>
				{coverage?.persisted ? (
					<span className="log-capture" title={coverage.path}>
						full history on disk
					</span>
				) : coverage ? (
					<span className="log-capture" title="No writable data directory; only the in-memory tail is searchable">
						memory only
					</span>
				) : null}
				{error && records.length > 0 ? <span className="error">{error}</span> : null}
			</p>
		</div>
	)
}
