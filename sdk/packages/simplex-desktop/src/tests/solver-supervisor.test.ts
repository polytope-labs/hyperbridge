import { createServer, type Server } from "node:http"
import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it, vi } from "vitest"
import { socketPathFor } from "../desktop-paths"
import {
	holdsMachineAwake,
	probeSolverStatus,
	sendSolverAction,
	SolverSupervisor,
	type SolverStatus,
} from "../solver-supervisor"

const cleanups: Array<() => Promise<void> | void> = []

afterEach(async () => {
	for (const cleanup of cleanups.splice(0).reverse()) await cleanup()
})

async function fixture(): Promise<{
	socketPath: string
	setPaused: (paused: boolean) => void
	actions: string[]
}> {
	const directory = mkdtempSync(join(tmpdir(), "simplex-supervisor-"))
	const socketPath = socketPathFor(directory)
	let paused = false
	const actions: string[] = []
	const server = createServer((request, response) => {
		if (request.url === "/health") {
			response.writeHead(200, { "content-type": "application/json" })
			response.end(JSON.stringify({ status: "ok", mode: "operator" }))
			return
		}
		if (request.url === "/api/status") {
			response.writeHead(200, { "content-type": "application/json" })
			response.end(JSON.stringify({ paused }))
			return
		}
		if (request.method === "POST" && /^\/api\/(pause|resume|stop)$/.test(request.url ?? "")) {
			if (request.headers["x-simplex-ui"] !== "1") {
				response.writeHead(403).end()
				return
			}
			actions.push(request.url ?? "")
			response.writeHead(202).end()
			return
		}
		response.writeHead(404).end()
	})
	await listen(server, socketPath)
	cleanups.push(async () => {
		await close(server)
		rmSync(directory, { recursive: true, force: true })
	})
	return { socketPath, setPaused: (value) => (paused = value), actions }
}

function listen(server: Server, socketPath: string): Promise<void> {
	return new Promise((resolve, reject) => {
		server.once("error", reject)
		server.listen(socketPath, resolve)
	})
}

function close(server: Server): Promise<void> {
	return new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())))
}

describe("solver supervision", () => {
	it("reads running and paused states through the private socket", async () => {
		const server = await fixture()
		expect(await probeSolverStatus(server.socketPath)).toEqual({ state: "running" })
		server.setPaused(true)
		expect(await probeSolverStatus(server.socketPath)).toEqual({ state: "paused" })
	})

	it("sends guarded pause, resume, and stop actions", async () => {
		const server = await fixture()
		for (const action of ["pause", "resume", "stop"] as const) {
			await sendSolverAction(server.socketPath, action)
		}
		expect(server.actions).toEqual(["/api/pause", "/api/resume", "/api/stop"])
	})

	it("reports a missing socket as stopped", async () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-supervisor-missing-"))
		cleanups.push(() => rmSync(directory, { recursive: true, force: true }))
		expect(await probeSolverStatus(socketPathFor(directory))).toEqual({ state: "stopped", detail: "absent" })
	})

	it("deduplicates concurrent probes and emits only status changes", async () => {
		let resolveProbe: ((status: SolverStatus) => void) | undefined
		const probe = vi.fn(
			() =>
				new Promise<SolverStatus>((resolve) => {
					resolveProbe = resolve
				}),
		)
		const onChange = vi.fn()
		const supervisor = new SolverSupervisor({ socketPath: "ignored", probe, onChange })
		const first = supervisor.pollNow()
		const second = supervisor.pollNow()
		expect(probe).toHaveBeenCalledTimes(1)
		resolveProbe?.({ state: "running" })
		await expect(first).resolves.toEqual({ state: "running" })
		await expect(second).resolves.toEqual({ state: "running" })
		expect(onChange).toHaveBeenCalledTimes(1)
	})

	it("holds the machine awake only while actively filling", () => {
		expect(holdsMachineAwake({ state: "running" })).toBe(true)
		expect(holdsMachineAwake({ state: "paused" })).toBe(false)
		expect(holdsMachineAwake({ state: "stopped" })).toBe(false)
	})
})
