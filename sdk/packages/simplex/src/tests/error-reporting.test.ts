import { describe, expect, it } from "vitest"
import {
	errorReportingDisabled,
	scrubBreadcrumb,
	scrubEvent,
	scrubText,
	sentryIngestOrigin,
} from "@/config/error-reporting"

const MNEMONIC = "abandon ability able about above absent absorb abstract absurd abuse access accident"
const PRIVATE_KEY = `0x${"4f".repeat(32)}`
const ADDRESS = "0x1111111111111111111111111111111111111111"

describe("error reporting", () => {
	it("treats only explicit opt-out values as disabled", () => {
		for (const value of ["off", "OFF", "0", "false", "no", " off "]) {
			expect(errorReportingDisabled(value), value).toBe(true)
		}
		for (const value of [undefined, "", "on", "1", "true"]) {
			expect(errorReportingDisabled(value), String(value)).toBe(false)
		}
	})

	it("derives the ingest origin from an HTTPS DSN only", () => {
		expect(sentryIngestOrigin("https://key@o1.ingest.us.sentry.io/2")).toBe("https://o1.ingest.us.sentry.io")
		expect(sentryIngestOrigin("http://key@o1.ingest.us.sentry.io/2")).toBeUndefined()
		expect(sentryIngestOrigin("not a dsn")).toBeUndefined()
	})

	it("redacts recovery phrases, key material and provider URL paths", () => {
		const scrubbed = scrubText(
			`rpc https://eth-mainnet.g.alchemy.com/v2/abcdefKEY?x=1 failed for ${ADDRESS} with ${PRIVATE_KEY}; phrase: ${MNEMONIC}`,
		)
		expect(scrubbed).toBe(
			`rpc https://eth-mainnet.g.alchemy.com/[redacted]?[redacted] failed for ${ADDRESS} with [redacted]; phrase: [redacted]`,
		)
	})

	it("keeps local solver routes and ordinary sentences readable", () => {
		expect(scrubText("GET http://127.0.0.1:8686/api/status?token=abc")).toBe(
			"GET http://127.0.0.1:8686/api/status?[redacted]",
		)
		const sentence = "the solver could not reach the bundler before the order deadline passed for this chain"
		expect(scrubText(sentence)).toBe(sentence)
	})

	it("scrubs free text but keeps identifiers and stack frames", () => {
		const eventId = "042cb5f47baf45248fae745717a6249f"
		const frames = [{ filename: "app:///dist/main.js", abs_path: `simplex://local/assets/${"a".repeat(40)}.js` }]
		const event = scrubEvent({
			event_id: eventId,
			message: `failed ${PRIVATE_KEY}`,
			exception: { values: [{ value: MNEMONIC, stacktrace: { frames } }] },
			breadcrumbs: [{ message: PRIVATE_KEY, data: { url: "https://rpc.example/v1/secret" } }],
			extra: { nested: [PRIVATE_KEY] },
			request: {
				url: "https://example.com/pair#secret",
				headers: { "User-Agent": "Chrome", Cookie: "a=b" },
				data: "body",
			},
		})
		expect(event.event_id).toBe(eventId)
		expect(event.message).toBe("failed [redacted]")
		expect(event.exception.values[0].value).toBe("[redacted]")
		expect(event.exception.values[0].stacktrace.frames).toEqual(frames)
		expect(event.breadcrumbs[0]).toEqual({
			message: "[redacted]",
			data: { url: "https://rpc.example/[redacted]" },
		})
		expect(event.extra).toEqual({ nested: ["[redacted]"] })
		expect(event.request).toEqual({
			url: "https://example.com/[redacted]?[redacted]",
			headers: { "User-Agent": "Chrome" },
		})
	})

	it("scrubs breadcrumbs on their own", () => {
		expect(scrubBreadcrumb({ message: `sent ${PRIVATE_KEY}` })).toEqual({ message: "sent [redacted]" })
	})
})
