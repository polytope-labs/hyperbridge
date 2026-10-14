// @vitest-environment jsdom
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest"
import * as Sentry from "@sentry/react"

const { send } = vi.hoisted(() => ({ send: vi.fn(async (_envelope: unknown) => ({ statusCode: 200 })) }))

vi.mock("@sentry/react", async (importOriginal) => {
	const actual = await importOriginal<typeof import("@sentry/react")>()
	return {
		...actual,
		init: vi.fn((options: Parameters<typeof actual.init>[0]) =>
			actual.init({ ...options, transport: () => ({ send, flush: async () => true }) }),
		),
	}
})

let startErrorReporting: typeof import("./error-reporting").startErrorReporting

beforeAll(async () => {
	vi.stubGlobal("__SIMPLEX_UI_SENTRY_DSN__", "https://public@example.com/1")
	;({ startErrorReporting } = await import("./error-reporting"))
})

beforeEach(() => vi.clearAllMocks())

afterEach(async () => {
	await Sentry.close(1_000)
})

afterAll(() => {
	vi.unstubAllGlobals()
})

describe("UI reporting permission", () => {
	it.each([
		["opt-out", () => Promise.resolve(Response.json({ errorReporting: false }))],
		["unavailable solver", () => Promise.reject(new Error("offline"))],
		["failed health response", () => Promise.resolve(new Response(null, { status: 503 }))],
	])("sends no envelope for %s", async (_name, fetchHealth) => {
		vi.stubGlobal("fetch", vi.fn(fetchHealth))
		await startErrorReporting()
		Sentry.captureException(new Error("suppressed"))
		await Sentry.flush(1_000)
		expect(Sentry.init).not.toHaveBeenCalled()
		expect(send).not.toHaveBeenCalled()
	})

	it("waits for permission before starting sessions or sending scrubbed errors", async () => {
		let allow!: (response: Response) => void
		vi.stubGlobal(
			"fetch",
			vi.fn(
				() =>
					new Promise<Response>((resolve) => {
						allow = resolve
					}),
			),
		)
		const started = startErrorReporting()
		expect(Sentry.init).not.toHaveBeenCalled()
		expect(send).not.toHaveBeenCalled()
		allow(Response.json({ status: "ok" }))
		await started
		expect(Sentry.init).toHaveBeenCalledOnce()
		Sentry.captureException(new Error(`secret ${"42".repeat(32)}`))
		await Sentry.flush(1_000)
		const envelopes = JSON.stringify(send.mock.calls)
		expect(envelopes).toContain("event")
		expect(envelopes).toContain("[redacted]")
		expect(envelopes).not.toContain("42".repeat(32))
	})
})
