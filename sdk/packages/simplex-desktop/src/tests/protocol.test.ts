import { randomUUID } from "node:crypto"
import { existsSync, mkdtempSync, unlinkSync } from "node:fs"
import { createServer, type RequestListener, type Server } from "node:http"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it } from "vitest"
import { proxyToSimplex } from "../protocol"

describe("simplex protocol proxy", () => {
	let server: Server | undefined
	let socketPath: string | undefined

	afterEach(async () => {
		if (server) await new Promise<void>((resolve) => server!.close(() => resolve()))
		if (socketPath && existsSync(socketPath)) unlinkSync(socketPath)
		server = undefined
		socketPath = undefined
	})

	async function listen(handler: RequestListener) {
		socketPath =
			process.platform === "win32"
				? `\\\\.\\pipe\\simplex-proxy-${process.pid}-${randomUUID()}`
				: join(mkdtempSync(join(tmpdir(), "simplex-proxy-")), "ui.sock")
		server = createServer(handler)
		await new Promise<void>((resolve, reject) => {
			server!.once("error", reject)
			server!.listen(socketPath, resolve)
		})
		return socketPath
	}

	it("preserves a GET path, query, status and response headers", async () => {
		const path = await listen((request, response) => {
			response.writeHead(206, { "Content-Type": "application/json", "X-Upstream": "simplex" })
			response.end(JSON.stringify({ path: request.url }))
		})
		const response = await proxyToSimplex(new Request("simplex://local/api/activity?before=42"), path)
		expect(response.status).toBe(206)
		expect(response.headers.get("content-type")).toBe("application/json")
		expect(response.headers.get("x-upstream")).toBe("simplex")
		expect(await response.json()).toEqual({ path: "/api/activity?before=42" })
	})

	it("preserves status, headers, request headers and a POST body", async () => {
		const path = await listen((request, response) => {
			let body = ""
			request.setEncoding("utf8")
			request.on("data", (chunk: Buffer | string) => (body += chunk.toString()))
			request.on("end", () => {
				response.writeHead(201, { "Content-Type": "application/json", "X-Upstream": "simplex" })
				response.end(JSON.stringify({ body, csrf: request.headers["x-simplex-ui"], path: request.url }))
			})
		})
		const request = new Request("simplex://local/api/setup?step=one", {
			method: "POST",
			headers: { "Content-Type": "text/plain", "X-Simplex-UI": "1" },
			body: "byte-identical",
		})
		const response = await proxyToSimplex(request, path)
		expect(response.status).toBe(201)
		expect(response.headers.get("content-type")).toBe("application/json")
		expect(response.headers.get("x-upstream")).toBe("simplex")
		expect(await response.json()).toEqual({ body: "byte-identical", csrf: "1", path: "/api/setup?step=one" })
	})

	it("streams upstream chunks without buffering", async () => {
		const path = await listen((_request, response) => {
			response.writeHead(200, { "Content-Type": "text/event-stream" })
			response.write(":ok\n\n")
			setTimeout(() => response.end("data: next\n\n"), 10)
		})
		const response = await proxyToSimplex(new Request("simplex://local/api/events"), path)
		const reader = response.body!.getReader()
		const first = await reader.read()
		expect(new TextDecoder().decode(first.value)).toBe(":ok\n\n")
		const second = await reader.read()
		expect(new TextDecoder().decode(second.value)).toBe("data: next\n\n")
	})

	it("closes the upstream stream when Chromium cancels", async () => {
		let upstreamClosed!: () => void
		const closed = new Promise<void>((resolve) => {
			upstreamClosed = resolve
		})
		const path = await listen((_request, response) => {
			response.writeHead(200, { "Content-Type": "text/event-stream" })
			response.write(":connected\n\n")
			response.once("close", upstreamClosed)
		})
		const controller = new AbortController()
		const response = await proxyToSimplex(
			new Request("simplex://local/api/events", { signal: controller.signal }),
			path,
		)
		expect(response.status).toBe(200)
		controller.abort()
		await expect(closed).resolves.toBeUndefined()
	})

	it("rejects transport failures instead of returning an HTTP error", async () => {
		const missing =
			process.platform === "win32"
				? `\\\\.\\pipe\\missing-simplex-${process.pid}-${randomUUID()}`
				: join(tmpdir(), `missing-simplex-${process.pid}.sock`)
		await expect(proxyToSimplex(new Request("simplex://local/api/events"), missing)).rejects.toMatchObject({
			code: "ENOENT",
		})
	})

	it("rejects every authority except simplex://local", async () => {
		await expect(proxyToSimplex(new Request("simplex://other/"), "/tmp/unused.sock")).rejects.toThrow(
			/unexpected Simplex origin/,
		)
	})
})
