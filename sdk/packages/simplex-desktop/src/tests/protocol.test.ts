import { randomUUID } from "node:crypto"
import { existsSync, mkdtempSync, mkdirSync, rmSync, unlinkSync, writeFileSync } from "node:fs"
import { createServer, type RequestListener, type Server } from "node:http"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it } from "vitest"
import { handleSimplexProtocol, proxyToSimplex } from "../protocol"

describe("simplex protocol proxy", () => {
	let server: Server | undefined
	let socketPath: string | undefined
	const temporaryDirectories: string[] = []

	afterEach(async () => {
		if (server) await new Promise<void>((resolve) => server!.close(() => resolve()))
		if (socketPath && existsSync(socketPath)) unlinkSync(socketPath)
		for (const directory of temporaryDirectories.splice(0)) rmSync(directory, { recursive: true, force: true })
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

	function uiFixture() {
		const directory = mkdtempSync(join(tmpdir(), "simplex-desktop-ui-"))
		temporaryDirectories.push(directory)
		mkdirSync(join(directory, "assets"))
		writeFileSync(join(directory, "index.html"), '<div id="root">packaged desktop UI</div>')
		writeFileSync(join(directory, "assets", "app.js"), "globalThis.simplexDesktop = true")
		return directory
	}

	it("serves the UI bundled with Electron instead of a stale solver UI", async () => {
		let upstreamRequests = 0
		const socketPath = await listen((_request, response) => {
			upstreamRequests += 1
			response.writeHead(200, { "Content-Type": "text/html" })
			response.end("<h1>UI not built</h1>")
		})
		const response = await handleSimplexProtocol(new Request("simplex://local/"), {
			socketPath,
			uiDistDir: uiFixture(),
			desktopVersion: "new-desktop",
		})

		expect(response.status).toBe(200)
		expect(response.headers.get("content-type")).toBe("text/html; charset=utf-8")
		expect(await response.text()).toContain("packaged desktop UI")
		expect(upstreamRequests).toBe(0)
	})

	it("serves bundled assets and falls back to its index for client routes", async () => {
		const uiDistDir = uiFixture()
		const asset = await handleSimplexProtocol(new Request("simplex://local/assets/app.js"), {
			socketPath: "/tmp/unused.sock",
			uiDistDir,
		})
		const route = await handleSimplexProtocol(new Request("simplex://local/orders/active"), {
			socketPath: "/tmp/unused.sock",
			uiDistDir,
		})

		expect(asset.headers.get("cache-control")).toBe("public, max-age=31536000, immutable")
		expect(await asset.text()).toContain("simplexDesktop")
		expect(await route.text()).toContain("packaged desktop UI")
	})

	it("continues proxying API requests to the running solver", async () => {
		const socketPath = await listen((request, response) => {
			response.writeHead(200, { "Content-Type": "application/json" })
			response.end(JSON.stringify({ path: request.url, version: "old-solver" }))
		})
		const response = await handleSimplexProtocol(new Request("simplex://local/api/status"), {
			socketPath,
			uiDistDir: uiFixture(),
			desktopVersion: "new-desktop",
		})

		expect(response.headers.get("x-simplex-desktop-version")).toBe("new-desktop")
		expect(await response.json()).toEqual({ path: "/api/status", version: "old-solver" })
	})

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

	it("exposes the immutable desktop version without changing the upstream body", async () => {
		const path = await listen((_request, response) => {
			response.writeHead(200, { "Content-Type": "application/json" })
			response.end(JSON.stringify({ version: "old-solver" }))
		})
		const response = await proxyToSimplex(new Request("simplex://local/api/status"), path, undefined, "new-desktop")
		expect(response.headers.get("x-simplex-desktop-version")).toBe("new-desktop")
		expect(await response.json()).toEqual({ version: "old-solver" })
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
