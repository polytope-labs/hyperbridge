import { describe, expect, it, vi } from "vitest"
import { consumeNotificationFrames, desktopNotificationUrl } from "../notification-client"

describe("desktop notification stream", () => {
	it("buffers split SSE frames and emits complete notifications", () => {
		const notify = vi.fn()
		const first = consumeNotificationFrames(':ok\n\ndata: {"title":"Low liquidity","body":"$12 remains",', notify)
		expect(notify).not.toHaveBeenCalled()
		const remainder = consumeNotificationFrames(`${first}"tag":"low","url":"./"}\n\n`, notify)
		expect(remainder).toBe("")
		expect(notify).toHaveBeenCalledWith({
			title: "Low liquidity",
			body: "$12 remains",
			tag: "low",
			url: "./",
		})
	})

	it("ignores malformed and incomplete payloads without losing later frames", () => {
		const notify = vi.fn()
		consumeNotificationFrames(
			'data: nope\n\ndata: {"title":"missing body"}\n\ndata: {"title":"Ready","body":"Works"}\n\n',
			notify,
		)
		expect(notify).toHaveBeenCalledOnce()
		expect(notify).toHaveBeenCalledWith({ title: "Ready", body: "Works", tag: "simplex-alert", url: "./" })
	})

	it("opens notification routes only inside the trusted desktop dashboard", () => {
		expect(desktopNotificationUrl("./orders")).toBe("simplex://local/orders")
		expect(desktopNotificationUrl("https://example.com/phish")).toBe("simplex://local/")
		expect(desktopNotificationUrl("simplex://remote/orders")).toBe("simplex://local/")
		expect(desktopNotificationUrl("not a url")).toBe("simplex://local/not%20a%20url")
	})
})
