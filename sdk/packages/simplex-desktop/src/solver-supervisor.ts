import { request as httpRequest } from "node:http"
import type { SolverWork } from "@hyperbridge/simplex"
import { probeHealth } from "./daemon"

export type SolverStatus =
	| { state: "starting"; pid?: number }
	| { state: "setup"; pid?: number; version?: string }
	| { state: "running"; pid?: number; version?: string; work?: SolverWork }
	| { state: "paused"; pid?: number; version?: string; work?: SolverWork }
	| { state: "stopping"; pid?: number }
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
	if (health.state === "starting") return { state: "starting", pid: health.pid }
	if (health.state === "stopping") return { state: "stopping", pid: health.pid }
	if (health.state === "spawnable") return { state: "stopped", detail: health.reason }
	if (health.state === "occupied" || health.state === "unavailable") {
		return { state: "unreachable", detail: health.detail }
	}
	try {
		const response = await socketRequest(socketPath, "/api/status", "GET")
		if (response.status !== 200) {
			return { state: "unreachable", detail: `/api/status returned ${response.status}` }
		}
		const status = JSON.parse(response.body) as {
			mode?: unknown
			paused?: unknown
			version?: unknown
			work?: unknown
		}
		const version = typeof status.version === "string" ? status.version : undefined
		if (health.mode === "init") {
			if (status.mode !== "init") {
				return { state: "unreachable", detail: "/api/status did not return init mode" }
			}
			return { state: "setup", pid: health.pid, version }
		}
		if (typeof status.paused !== "boolean") {
			return { state: "unreachable", detail: "/api/status did not return a pause state" }
		}
		const work = parseSolverWork(status.work)
		return { state: status.paused ? "paused" : "running", pid: health.pid, version, work }
	} catch (error) {
		return { state: "unreachable", detail: error instanceof Error ? error.message : String(error) }
	}
}

function parseSolverWork(value: unknown): SolverWork | undefined {
	if (!value || typeof value !== "object") return undefined
	const candidate = value as Record<keyof SolverWork, unknown>
	const fields: Array<keyof SolverWork> = [
		"queuedEvaluations",
		"evaluating",
		"queuedFills",
		"activeFills",
		"retractions",
		"rebalancing",
	]
	if (!fields.every((field) => typeof candidate[field] === "number" && candidate[field] >= 0)) return undefined
	return {
		queuedEvaluations: candidate.queuedEvaluations as number,
		evaluating: candidate.evaluating as number,
		queuedFills: candidate.queuedFills as number,
		activeFills: candidate.activeFills as number,
		retractions: candidate.retractions as number,
		rebalancing: candidate.rebalancing as number,
	}
}

export function solverIsIdle(status: SolverStatus): boolean {
	if (status.state !== "running" && status.state !== "paused") return false
	const work = status.work
	return Boolean(
		work &&
			(status.state === "paused" || work.queuedEvaluations === 0) &&
			work.evaluating === 0 &&
			work.queuedFills === 0 &&
			work.activeFills === 0 &&
			work.retractions === 0 &&
			work.rebalancing === 0,
	)
}

export function solverVersion(status: SolverStatus): string | undefined {
	return status.state === "setup" || status.state === "running" || status.state === "paused"
		? status.version
		: undefined
}

export function solverHasVersionSkew(status: SolverStatus, appVersion: string): boolean {
	return (
		(status.state === "setup" || status.state === "running" || status.state === "paused") &&
		status.version !== appVersion
	)
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

export function shouldNotifySolverFailure(
	previous: SolverStatus,
	next: SolverStatus,
	intentionalStop: boolean,
): boolean {
	if (intentionalStop) return false
	return (
		(previous.state === "setup" || previous.state === "running" || previous.state === "paused") &&
		(next.state === "unreachable" || (next.state === "stopped" && next.detail === "stale"))
	)
}

/** A detached solver can finish draining after Electron has accepted its stop. */
export function stopRequestAccepted(status: SolverStatus): boolean {
	return status.state === "stopping" || status.state === "stopped"
}

function sameStatus(left: SolverStatus, right: SolverStatus): boolean {
	return JSON.stringify(left) === JSON.stringify(right)
}

export class SolverSupervisor {
	private timer?: NodeJS.Timeout
	private polling?: Promise<SolverStatus>
	private current: SolverStatus = { state: "starting" }
	private consecutiveFailures = 0

	constructor(
		private readonly options: {
			socketPath: string
			onChange: (next: SolverStatus, previous: SolverStatus) => void
			probe?: (socketPath: string) => Promise<SolverStatus>
			intervalMs?: number
			failureThreshold?: number
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
			return this.applyProbe(next)
		} finally {
			this.polling = undefined
		}
	}

	private applyProbe(next: SolverStatus): SolverStatus {
		const wasLive = this.current.state === "running" || this.current.state === "paused"
		const failed = next.state === "stopped" || next.state === "unreachable"
		if (wasLive && failed) {
			this.consecutiveFailures += 1
			if (this.consecutiveFailures < (this.options.failureThreshold ?? 2)) return this.current
		} else {
			this.consecutiveFailures = 0
		}
		this.setStatus(next)
		return this.current
	}

	start(): void {
		if (this.timer) return
		const poll = () =>
			void this.pollNow().catch((error) =>
				this.applyProbe({
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
