import { request as httpRequest } from "node:http"
import { probeHealth } from "./daemon"

export type SolverStatus =
	| { state: "starting" }
	| { state: "setup" }
	| { state: "running" }
	| { state: "paused" }
	| { state: "stopped"; detail?: string }
	| { state: "unreachable"; detail: string }

export type SolverAction = "pause" | "resume" | "stop"

const MAX_RESPONSE_BYTES = 64 * 1024

function socketRequest(
	socketPath: string,
	path: string,
	method: "GET" | "POST",
	timeoutMs = 1_000,
): Promise<{ status: number; body: string }> {
	return new Promise((resolve, reject) => {
		const request = httpRequest(
			{
				socketPath,
				path,
				method,
				headers: method === "POST" ? { "X-Simplex-UI": "1" } : undefined,
			},
			(response) => {
				const chunks: Buffer[] = []
				let bytes = 0
				response.on("data", (chunk: Buffer) => {
					bytes += chunk.length
					if (bytes > MAX_RESPONSE_BYTES) {
						request.destroy(new Error(`Simplex ${path} response exceeded ${MAX_RESPONSE_BYTES} bytes`))
						return
					}
					chunks.push(chunk)
				})
				response.on("end", () =>
					resolve({ status: response.statusCode ?? 0, body: Buffer.concat(chunks).toString("utf8") }),
				)
			},
		)
		request.setTimeout(timeoutMs, () => request.destroy(new Error(`Simplex ${path} request timed out`)))
		request.on("error", reject)
		request.end()
	})
}

/** The state the native shell displays, read only through the private socket. */
export async function probeSolverStatus(socketPath: string): Promise<SolverStatus> {
	const health = await probeHealth(socketPath)
	if (health.state === "spawnable") return { state: "stopped", detail: health.reason }
	if (health.state === "occupied" || health.state === "unavailable") {
		return { state: "unreachable", detail: health.detail }
	}
	if (health.mode === "init") return { state: "setup" }

	try {
		const response = await socketRequest(socketPath, "/api/status", "GET")
		if (response.status !== 200) {
			return { state: "unreachable", detail: `/api/status returned ${response.status}` }
		}
		const status = JSON.parse(response.body) as { paused?: unknown }
		if (typeof status.paused !== "boolean") {
			return { state: "unreachable", detail: "/api/status did not return a pause state" }
		}
		return { state: status.paused ? "paused" : "running" }
	} catch (error) {
		return { state: "unreachable", detail: error instanceof Error ? error.message : String(error) }
	}
}

export async function sendSolverAction(socketPath: string, action: SolverAction): Promise<void> {
	const response = await socketRequest(socketPath, `/api/${action}`, "POST", 5_000)
	if (response.status < 200 || response.status >= 300) {
		throw new Error(`Simplex could not ${action}: ${response.status}${response.body ? ` ${response.body}` : ""}`)
	}
}

export function holdsMachineAwake(status: SolverStatus): boolean {
	return status.state === "running"
}

function sameStatus(left: SolverStatus, right: SolverStatus): boolean {
	return (
		left.state === right.state &&
		("detail" in left ? left.detail : undefined) === ("detail" in right ? right.detail : undefined)
	)
}

export class SolverSupervisor {
	private timer?: NodeJS.Timeout
	private polling?: Promise<SolverStatus>
	private current: SolverStatus = { state: "starting" }

	constructor(
		private readonly options: {
			socketPath: string
			onChange: (next: SolverStatus, previous: SolverStatus) => void
			probe?: (socketPath: string) => Promise<SolverStatus>
			intervalMs?: number
		},
	) {}

	get status(): SolverStatus {
		return this.current
	}

	setStatus(next: SolverStatus): void {
		if (sameStatus(this.current, next)) return
		const previous = this.current
		this.current = next
		this.options.onChange(next, previous)
	}

	async pollNow(): Promise<SolverStatus> {
		if (this.polling) return this.polling
		this.polling = (this.options.probe ?? probeSolverStatus)(this.options.socketPath)
		try {
			const next = await this.polling
			this.setStatus(next)
			return next
		} finally {
			this.polling = undefined
		}
	}

	start(): void {
		if (this.timer) return
		const poll = () =>
			void this.pollNow().catch((error) =>
				this.setStatus({
					state: "unreachable",
					detail: error instanceof Error ? error.message : String(error),
				}),
			)
		poll()
		this.timer = setInterval(poll, this.options.intervalMs ?? 3_000)
		this.timer.unref()
	}

	stop(): void {
		if (this.timer) clearInterval(this.timer)
		this.timer = undefined
	}
}
