import { createServer, type Server } from "node:http"
import { linkSync, mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { createServer as createNetServer } from "node:net"
import { afterEach, describe, expect, it, vi } from "vitest"
import { socketPathFor } from "../desktop-paths"
import {
	depositsNeedConfirming,
	holdsMachineAwake,
	probeSolverStatus,
	sendSolverAction,
	settleBidsHoldingDeposits,
	shouldNotifySolverFailure,
	solverHasVersionSkew,
	solverIsIdle,
	stopRequestAccepted,
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
	setHealthStatus: (status: "ok" | "stopping") => void
	/** Fills the solver still has in flight. */
	setActiveFills: (count: number) => void
	/** Bids that hold a deposit, as `/api/activity/bids` reports them. */
	setDeposits: (count: number) => void
	actions: string[]
}> {
	const directory = mkdtempSync(join(tmpdir(), "simplex-supervisor-"))
	const socketPath = socketPathFor(directory)
	let paused = false
	let healthStatus: "ok" | "stopping" = "ok"
	let activeFills = 0
	let deposits = 0
	const actions: string[] = []
	const server = createServer((request, response) => {
		if (request.url === "/health") {
			response.writeHead(200, { "content-type": "application/json" })
			response.end(JSON.stringify({ status: healthStatus, mode: "operator", pid: process.pid }))
			return
		}
		if (request.url === "/api/status") {
			response.writeHead(200, { "content-type": "application/json" })
			response.end(
				JSON.stringify({
					paused,
					version: "0.16.2",
					work: {
						queuedEvaluations: 0,
						evaluating: 0,
						queuedFills: 0,
						activeFills,
						retractions: 0,
						rebalancing: 0,
					},
				}),
			)
			return
		}
		if (request.url === "/api/activity/bids?limit=1") {
			response.writeHead(200, { "content-type": "application/json" })
			response.end(JSON.stringify({ stats: { pendingRetraction: deposits } }))
			return
		}
		if (request.method === "POST" && /^\/api\/(pause|resume|stop)$/.test(request.url ?? "")) {
			if (request.headers["x-simplex-ui"] !== "1") {
				response.writeHead(403).end()
				return
			}
			actions.push(request.url ?? "")
			if (request.url === "/api/pause") paused = true
			if (request.url === "/api/resume") paused = false
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
	return {
		socketPath,
		setPaused: (value) => (paused = value),
		setHealthStatus: (value) => (healthStatus = value),
		setActiveFills: (value) => (activeFills = value),
		setDeposits: (value) => (deposits = value),
		actions,
	}
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
	it.each(["/health", "/api/status"])(
		"resumes polling after an interrupted %s response",
		async (brokenPath) => {
			const directory = mkdtempSync(join(tmpdir(), "simplex-interrupted-"))
			const socketPath = socketPathFor(directory)
			let interrupt = true
			const server = createServer((request, response) => {
				if (interrupt && request.url === brokenPath) {
					response.writeHead(200, { "Content-Length": "1000" })
					response.write("{", () => response.destroy())
					return
				}
				response.end(
					JSON.stringify(
						request.url === "/health"
							? { status: "ok", mode: "operator", pid: process.pid }
							: { paused: false, version: "0.17.4" },
					),
				)
			})
			await listen(server, socketPath)
			cleanups.push(async () => {
				server.closeAllConnections()
				await close(server)
				rmSync(directory, { recursive: true, force: true })
			})
			const supervisor = new SolverSupervisor({ socketPath, onChange() {}, failureThreshold: 1 })
			supervisor.setStatus({ state: "running" })
			await expect(supervisor.pollNow()).resolves.toMatchObject({ state: "unreachable" })
			interrupt = false
			await expect(supervisor.pollNow()).resolves.toMatchObject({ state: "running", version: "0.17.4" })
		},
		2_000,
	)

	it("reads running and paused states through the private socket", async () => {
		const server = await fixture()
		expect(await probeSolverStatus(server.socketPath)).toEqual({
			state: "running",
			pid: process.pid,
			version: "0.16.2",
			work: {
				queuedEvaluations: 0,
				evaluating: 0,
				queuedFills: 0,
				activeFills: 0,
				retractions: 0,
				rebalancing: 0,
			},
		})
		server.setPaused(true)
		expect(await probeSolverStatus(server.socketPath)).toMatchObject({ state: "paused", pid: process.pid })
	})

	it("reads setup version from a previous daemon that does not report a PID", async () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-supervisor-legacy-"))
		const socketPath = socketPathFor(directory)
		const server = createServer((request, response) => {
			response.writeHead(200, { "content-type": "application/json" })
			response.end(
				JSON.stringify(
					request.url === "/health"
						? { status: "ok", mode: "init" }
						: { mode: "init", version: "0.16.1", starting: false },
				),
			)
		})
		await listen(server, socketPath)
		cleanups.push(async () => {
			await close(server)
			rmSync(directory, { recursive: true, force: true })
		})

		expect(await probeSolverStatus(socketPath)).toEqual({ state: "setup", version: "0.16.1" })
	})

	it("reports stopping before the socket disappears", async () => {
		const server = await fixture()
		server.setHealthStatus("stopping")
		expect(await probeSolverStatus(server.socketPath)).toEqual({ state: "stopping", pid: process.pid })
	})

	it("sends guarded pause, resume, and stop actions", async () => {
		const server = await fixture()
		for (const action of ["pause", "resume", "stop"] as const) {
			await sendSolverAction(server.socketPath, action)
		}
		expect(server.actions).toEqual(["/api/pause", "/api/resume", "/api/stop"])
	})

	it("counts the bids holding deposits only once the solver is paused and has nothing in flight", async () => {
		const server = await fixture()
		server.setDeposits(1)
		server.setActiveFills(1)
		// The fill in flight lands a second bid before the solver goes quiet.
		let waits = 0
		const delay = async () => {
			waits += 1
			server.setDeposits(2)
			server.setActiveFills(0)
		}
		await expect(settleBidsHoldingDeposits(server.socketPath, { delay })).resolves.toEqual({
			count: 2,
			paused: true,
		})
		expect(server.actions).toEqual(["/api/pause"])
		expect(waits).toBe(1)
	})

	it("gives no settled count for a solver that does not go quiet in time", async () => {
		const server = await fixture()
		server.setDeposits(1)
		server.setActiveFills(1)
		await expect(
			settleBidsHoldingDeposits(server.socketPath, { timeoutMs: 0, delay: async () => {} }),
		).resolves.toEqual({ count: undefined, paused: true })
	})

	it("does not pause a solver that is already paused, and counts without one that is not up", async () => {
		const server = await fixture()
		server.setPaused(true)
		server.setDeposits(3)
		await expect(settleBidsHoldingDeposits(server.socketPath)).resolves.toEqual({ count: 3, paused: false })
		expect(server.actions).toEqual([])

		const empty = mkdtempSync(join(tmpdir(), "simplex-supervisor-none-"))
		cleanups.push(() => rmSync(empty, { recursive: true, force: true }))
		const missing = socketPathFor(empty)
		await expect(settleBidsHoldingDeposits(missing)).resolves.toEqual({ count: undefined, paused: false })
	})

	it("asks again only when the settled count says more than the operator was told", () => {
		// More bids hold deposits than the confirmation named.
		expect(depositsNeedConfirming(0, 1)).toBe(true)
		expect(depositsNeedConfirming(2, 3)).toBe(true)
		expect(depositsNeedConfirming(undefined, 1)).toBe(true)
		// The same or fewer, or nothing where it had already said it could not check.
		expect(depositsNeedConfirming(2, 2)).toBe(false)
		expect(depositsNeedConfirming(2, 0)).toBe(false)
		expect(depositsNeedConfirming(undefined, 0)).toBe(false)
		expect(depositsNeedConfirming(undefined, undefined)).toBe(false)
		// It named a count, and now there is none to check it against.
		expect(depositsNeedConfirming(0, undefined)).toBe(true)
	})

	it("reports a missing socket as stopped", async () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-supervisor-missing-"))
		cleanups.push(() => rmSync(directory, { recursive: true, force: true }))
		expect(await probeSolverStatus(socketPathFor(directory))).toEqual({ state: "stopped", detail: "absent" })
	})

	it("reports a Unix socket orphaned by a crash as stale", async () => {
		if (process.platform === "win32") return
		const directory = mkdtempSync(join(tmpdir(), "simplex-supervisor-stale-"))
		const bound = join(directory, "bound.sock")
		const orphan = join(directory, "orphan.sock")
		const server = createNetServer()
		await new Promise<void>((resolve, reject) => {
			server.once("error", reject)
			server.listen(bound, resolve)
		})
		linkSync(bound, orphan)
		await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())))
		cleanups.push(() => rmSync(directory, { recursive: true, force: true }))

		expect(await probeSolverStatus(orphan)).toEqual({ state: "stopped", detail: "stale" })
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

	it("updates work snapshots without rebuilding native menus", () => {
		const onChange = vi.fn()
		const supervisor = new SolverSupervisor({ socketPath: "ignored", onChange })
		const idle = {
			queuedEvaluations: 0,
			evaluating: 0,
			queuedFills: 0,
			activeFills: 0,
			retractions: 0,
			rebalancing: 0,
		}
		supervisor.setStatus({ state: "running", version: "0.16.2", work: idle })
		supervisor.setStatus({ state: "running", version: "0.16.2", work: { ...idle, activeFills: 1 } })

		expect(onChange).toHaveBeenCalledTimes(1)
		expect(supervisor.status).toEqual({
			state: "running",
			version: "0.16.2",
			work: { ...idle, activeFills: 1 },
		})
	})

	it("requires consecutive failures before leaving a live state", async () => {
		const probe = vi
			.fn<() => Promise<SolverStatus>>()
			.mockResolvedValueOnce({ state: "running" })
			.mockResolvedValueOnce({ state: "unreachable", detail: "slow poll" })
			.mockResolvedValueOnce({ state: "running" })
			.mockResolvedValueOnce({ state: "unreachable", detail: "slow poll" })
			.mockResolvedValueOnce({ state: "unreachable", detail: "still unavailable" })
		const onChange = vi.fn()
		const supervisor = new SolverSupervisor({ socketPath: "ignored", probe, onChange, failureThreshold: 2 })

		await supervisor.pollNow()
		await supervisor.pollNow()
		expect(supervisor.status).toEqual({ state: "running" })
		await supervisor.pollNow()
		await supervisor.pollNow()
		expect(supervisor.status).toEqual({ state: "running" })
		await supervisor.pollNow()
		expect(supervisor.status).toEqual({ state: "unreachable", detail: "still unavailable" })
	})

	it("holds the machine awake only while actively filling", () => {
		expect(holdsMachineAwake({ state: "running" })).toBe(true)
		expect(holdsMachineAwake({ state: "paused" })).toBe(false)
		expect(holdsMachineAwake({ state: "stopping" })).toBe(false)
		expect(holdsMachineAwake({ state: "stopped" })).toBe(false)
	})

	it("only reports idle when every authoritative work count is zero", () => {
		expect(
			solverIsIdle({
				state: "running",
				work: {
					queuedEvaluations: 0,
					evaluating: 0,
					queuedFills: 0,
					activeFills: 0,
					retractions: 0,
					rebalancing: 0,
				},
			}),
		).toBe(true)
		expect(
			solverIsIdle({
				state: "running",
				work: {
					queuedEvaluations: 1,
					evaluating: 0,
					queuedFills: 0,
					activeFills: 0,
					retractions: 0,
					rebalancing: 0,
				},
			}),
		).toBe(false)
		expect(
			solverIsIdle({
				state: "paused",
				work: {
					queuedEvaluations: 1,
					evaluating: 0,
					queuedFills: 0,
					activeFills: 0,
					retractions: 0,
					rebalancing: 0,
				},
			}),
		).toBe(true)
		expect(
			solverIsIdle({
				state: "paused",
				work: {
					queuedEvaluations: 0,
					evaluating: 0,
					queuedFills: 0,
					activeFills: 1,
					retractions: 0,
					rebalancing: 0,
				},
			}),
		).toBe(false)
		expect(
			solverIsIdle({
				state: "running",
				work: {
					queuedEvaluations: 0,
					evaluating: 0,
					queuedFills: 0,
					activeFills: 0,
					retractions: 0,
					rebalancing: 1,
				},
			}),
		).toBe(false)
		expect(solverIsIdle({ state: "running" })).toBe(false)
	})

	it("treats setup without a matching reported version as skewed", () => {
		expect(solverHasVersionSkew({ state: "setup", version: "0.16.2" }, "0.17.0")).toBe(true)
		expect(solverHasVersionSkew({ state: "setup" }, "0.17.0")).toBe(true)
		expect(solverHasVersionSkew({ state: "setup", version: "0.17.0" }, "0.17.0")).toBe(false)
	})

	it("notifies for crashes but not deliberate stops", () => {
		expect(shouldNotifySolverFailure({ state: "running" }, { state: "stopped", detail: "stale" }, false)).toBe(true)
		expect(shouldNotifySolverFailure({ state: "running" }, { state: "stopped", detail: "absent" }, false)).toBe(
			true,
		)
		expect(shouldNotifySolverFailure({ state: "running" }, { state: "stopped", detail: "absent" }, true)).toBe(
			false,
		)
		expect(shouldNotifySolverFailure({ state: "running" }, { state: "unreachable", detail: "crash" }, false)).toBe(
			true,
		)
		expect(shouldNotifySolverFailure({ state: "running" }, { state: "stopped", detail: "stale" }, true)).toBe(false)
		expect(shouldNotifySolverFailure({ state: "stopping" }, { state: "stopped", detail: "stale" }, false)).toBe(
			false,
		)
	})

	it("accepts a stopping daemon as a successful graceful-stop handoff", () => {
		expect(stopRequestAccepted({ state: "stopping" })).toBe(true)
		expect(stopRequestAccepted({ state: "stopped", detail: "absent" })).toBe(true)
		expect(stopRequestAccepted({ state: "running" })).toBe(false)
	})
})
