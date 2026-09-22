import { request as httpRequest, type ClientRequest, type IncomingMessage, type RequestOptions } from "node:http"
import type { OperatorNotification } from "@hyperbridge/simplex"

/** Resolve only solver-owned dashboard routes; alert payloads never navigate Electron elsewhere. */
export function desktopNotificationUrl(value: string): string {
	try {
		const target = new URL(value, "simplex://local/")
		if (
			target.protocol === "simplex:" &&
			target.hostname === "local" &&
			!target.username &&
			!target.password &&
			!target.port
		) {
			return target.href
		}
	} catch {
		// Fall through to the trusted dashboard root.
	}
	return "simplex://local/"
}

type HttpRequest = typeof httpRequest

export function consumeNotificationFrames(
	buffer: string,
	onNotification: (notification: OperatorNotification) => void,
): string {
	const frames = buffer.split("\n\n")
	const remainder = frames.pop() ?? ""
	for (const frame of frames) {
		const data = frame
			.split("\n")
			.filter((line) => line.startsWith("data:"))
			.map((line) => line.slice(5).trimStart())
			.join("\n")
		if (!data) continue
		try {
			const value = JSON.parse(data) as Partial<OperatorNotification>
			if (typeof value.title === "string" && typeof value.body === "string") {
				onNotification({
					title: value.title,
					body: value.body,
					tag: typeof value.tag === "string" ? value.tag : "simplex-alert",
					url: typeof value.url === "string" ? value.url : "./",
				})
			}
		} catch {
			// A broken frame must not take down the long-lived desktop listener.
		}
	}
	return remainder
}

/** Reconnecting SSE client for solver-originated native desktop alerts. */
export class DesktopNotificationClient {
	private request?: ClientRequest
	private response?: IncomingMessage
	private reconnectTimer?: NodeJS.Timeout
	private stopped = true

	constructor(
		private readonly options: {
			socketPath: string
			onNotification: (notification: OperatorNotification) => void
			request?: HttpRequest
			retryMs?: number
		},
	) {}

	start(): void {
		if (!this.stopped) return
		this.stopped = false
		this.connect()
	}

	stop(): void {
		this.stopped = true
		if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
		this.reconnectTimer = undefined
		this.response?.destroy()
		this.request?.destroy()
		this.response = undefined
		this.request = undefined
	}

	private connect(): void {
		if (this.stopped) return
		const requestImpl = this.options.request ?? httpRequest
		const options: RequestOptions = {
			socketPath: this.options.socketPath,
			path: "/api/notifications/stream",
			method: "GET",
			headers: { Accept: "text/event-stream" },
		}
		let settled = false
		const reconnect = () => {
			if (settled || this.stopped) return
			settled = true
			this.response = undefined
			this.request = undefined
			this.reconnectTimer = setTimeout(() => {
				this.reconnectTimer = undefined
				this.connect()
			}, this.options.retryMs ?? 5_000)
			this.reconnectTimer.unref()
		}
		this.request = requestImpl(options, (response) => {
			this.response = response
			if (response.statusCode !== 200) {
				response.resume()
				response.once("end", reconnect)
				return
			}
			response.setEncoding("utf8")
			let buffer = ""
			response.on("data", (chunk: string) => {
				buffer = consumeNotificationFrames(buffer + chunk, this.options.onNotification)
			})
			response.once("end", reconnect)
			response.once("error", reconnect)
			response.once("close", reconnect)
		})
		this.request.once("error", reconnect)
		this.request.end()
	}
}
