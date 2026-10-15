import { wordlist } from "@scure/bip39/wordlists/english"

/**
 * Error reporting shared by the web UI and the desktop shell. Builds only report
 * when CI baked a Sentry DSN into them; operators opt out with
 * `SIMPLEX_ERROR_REPORTING=off`. Everything here is browser-safe.
 */
export const ERROR_REPORTING_ENV = "SIMPLEX_ERROR_REPORTING"

export function errorReportingDisabled(value: string | undefined): boolean {
	return ["off", "0", "false", "no"].includes((value ?? "").trim().toLowerCase())
}

/** The ingest origin a renderer CSP must allow for this DSN, or undefined for a malformed one. */
export function sentryIngestOrigin(dsn: string): string | undefined {
	try {
		const url = new URL(dsn)
		return url.protocol === "https:" ? url.origin : undefined
	} catch {
		return undefined
	}
}

const MNEMONIC_WORDS = new Set(wordlist)
const MNEMONIC_MIN_WORDS = 12
const REDACTED = "[redacted]"
const LOOPBACK_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]", "local"])
const URL_PATTERN = /\b(?:https?|wss?|simplex|file):\/\/[^\s"'<>`]+/i
// Quoted paths may contain spaces. Match URLs first so local API routes remain readable.
const FILE_PATH_PATTERN = /(?<=["'])(?:[a-z]:[\\/]|\\\\|\/)[^"'<>`\r\n]+|(?:[a-z]:[\\/]|\\\\|(?<![\w/])\/)[^\s"'<>`]+/
const LOCATION_PATTERN = new RegExp(`${URL_PATTERN.source}|${FILE_PATH_PATTERN.source}`, "gi")
// Also catches phrases rendered as JSON word arrays or comma-separated words.
const WORD_RUN_PATTERN = /\b[a-z]+(?:[\s"',\[\]]+[a-z]+){11,}\b/gi
// Private keys, signatures, API tokens, base58/base64 secrets and long file paths
// (which name the operator's home directory). A 0x address stays readable.
const LONG_TOKEN_PATTERN = /[A-Za-z0-9+/_=-]{32,}/g
const ADDRESS_PATTERN = /^0x[0-9a-fA-F]{40}$/

function scrubUrl(raw: string): string {
	try {
		const url = new URL(raw)
		if (url.protocol === "file:") return REDACTED
		// RPC providers carry API keys in the path or query; local solver routes do not.
		const path = LOOPBACK_HOSTS.has(url.hostname) ? url.pathname : url.pathname === "/" ? "" : `/${REDACTED}`
		return `${url.protocol}//${url.host}${path}${url.search || url.hash ? `?${REDACTED}` : ""}`
	} catch {
		return REDACTED
	}
}

function containsMnemonic(run: string): boolean {
	let consecutive = 0
	for (const word of run.toLowerCase().split(/[\s"',\[\]]+/)) {
		consecutive = MNEMONIC_WORDS.has(word) ? consecutive + 1 : 0
		if (consecutive >= MNEMONIC_MIN_WORDS) return true
	}
	return false
}

/** Removes recovery phrases, key material and provider URLs from one string. */
export function scrubText(text: string): string {
	const withoutUrls = text.replace(LOCATION_PATTERN, (location) =>
		URL_PATTERN.test(location) ? scrubUrl(location) : REDACTED,
	)
	const withoutMnemonics = withoutUrls.replace(WORD_RUN_PATTERN, (run) => (containsMnemonic(run) ? REDACTED : run))
	return withoutMnemonics.replace(LONG_TOKEN_PATTERN, (token) => (ADDRESS_PATTERN.test(token) ? token : REDACTED))
}

interface ScrubbableBreadcrumb {
	message?: string
	data?: Record<string, unknown>
}

interface ScrubbableEvent {
	message?: string
	logentry?: { message?: string; params?: unknown[] }
	exception?: { values?: Array<{ value?: string }> }
	breadcrumbs?: ScrubbableBreadcrumb[]
	extra?: Record<string, unknown>
	request?: {
		url?: string
		query_string?: unknown
		data?: unknown
		cookies?: unknown
		headers?: Record<string, string>
	}
}

/** Scrubs a breadcrumb's message and data in place. */
export function scrubBreadcrumb<T extends ScrubbableBreadcrumb>(breadcrumb: T): T {
	if (breadcrumb.message) breadcrumb.message = scrubText(breadcrumb.message)
	if (breadcrumb.data) breadcrumb.data = scrubDeep(breadcrumb.data) as Record<string, unknown>
	return breadcrumb
}

/**
 * Scrubs the free-text parts of an event in place. Identifiers (event and
 * trace ids), release names and stack frames stay intact so Sentry can group
 * the event and resolve uploaded source maps.
 */
export function scrubEvent<T extends ScrubbableEvent>(event: T): T {
	if (event.message) event.message = scrubText(event.message)
	if (event.logentry) event.logentry = scrubDeep(event.logentry) as ScrubbableEvent["logentry"]
	for (const exception of event.exception?.values ?? []) {
		if (exception.value) exception.value = scrubText(exception.value)
	}
	for (const breadcrumb of event.breadcrumbs ?? []) scrubBreadcrumb(breadcrumb)
	if (event.extra) event.extra = scrubDeep(event.extra) as Record<string, unknown>
	if (event.request) {
		if (event.request.url) event.request.url = scrubText(event.request.url)
		delete event.request.query_string
		delete event.request.data
		delete event.request.cookies
		// Sentry derives the browser and OS from the user agent; no other header is needed.
		const userAgent = event.request.headers?.["User-Agent"]
		event.request.headers = userAgent ? { "User-Agent": userAgent } : undefined
	}
	return event
}

function scrubDeep(value: unknown, seen = new WeakSet<object>()): unknown {
	if (typeof value === "string") return scrubText(value)
	if (!value || typeof value !== "object") return value
	if (seen.has(value)) return REDACTED
	seen.add(value)
	if (Array.isArray(value)) {
		if (containsMnemonic(value.map((item) => (typeof item === "string" ? item : "")).join(" "))) return REDACTED
		return value.map((item) => scrubDeep(item, seen))
	}
	return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, scrubDeep(item, seen)]))
}
