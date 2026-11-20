import { createServer, type RequestListener } from "node:http"
import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, expect, it } from "vitest"
import { socketPathFor } from "../desktop-paths"
import { socketRequest } from "../socket-request"
import { probeHealth } from "../daemon"
import { sendSolverAction } from "../solver-supervisor"

const cleanups: Array<() => Promise<void>> = []
afterEach(async () => {
	for (const cleanup of cleanups.splice(0)) await cleanup()
})
async function listen(handler: RequestListener) {
	const dir = mkdtempSync(join(tmpdir(), "simplex-control-"))
	const socketPath = socketPathFor(dir)
	const server = createServer(handler)
	await new Promise<void>((resolve, reject) => {
		server.once("error", reject)
		server.listen(socketPath, resolve)
	})
	cleanups.push(async () => {
		server.closeAllConnections()
		await new Promise<void>((resolve) => server.close(() => resolve()))
		rmSync(dir, { recursive: true, force: true })
	})
	return socketPath
}

it("enforces a deadline even while a response keeps trickling in", async () => {
	const path = await listen((_req, res) => {
		res.writeHead(200)
		const timer = setInterval(() => res.write(" "), 5)
		res.once("close", () => clearInterval(timer))
	})
	await expect(socketRequest(path, "/health", "GET", 50)).rejects.toThrow(/timed out/)
}, 1_000)

it("bounds health responses as well as status responses", async () => {
	const path = await listen((_req, res) => res.end("x".repeat(65_537)))
	await expect(probeHealth(path)).resolves.toMatchObject({
		state: "unavailable",
		detail: expect.stringContaining("exceeded"),
	})
})

it("rejects interrupted stop responses instead of leaving the stop action pending", async () => {
	const path = await listen((_req, res) => {
		res.writeHead(202, { "Content-Length": "1000" })
		res.write("{", () => res.destroy())
	})
	await expect(sendSolverAction(path, "stop")).rejects.toThrow(/before completion/)
}, 1_000)
