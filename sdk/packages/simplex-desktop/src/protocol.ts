import { request as httpRequest, type IncomingHttpHeaders, type RequestOptions } from "node:http"
import { readFile, stat } from "node:fs/promises"
import { extname, resolve, sep } from "node:path"
import { Readable } from "node:stream"

type HttpRequest = typeof httpRequest

const MIME_TYPES: Record<string, string> = {
	".css": "text/css; charset=utf-8",
	".html": "text/html; charset=utf-8",
	".ico": "image/x-icon",
	".js": "text/javascript; charset=utf-8",
	".json": "application/json",
	".map": "application/json",
	".mjs": "text/javascript; charset=utf-8",
	".png": "image/png",
	".svg": "image/svg+xml",
	".txt": "text/plain; charset=utf-8",
	".webmanifest": "application/manifest+json",
	".webp": "image/webp",
	".woff2": "font/woff2",
}

function simplexUrl(request: Request): URL {
	const url = new URL(request.url)
	if (url.protocol !== "simplex:" || url.hostname !== "local" || url.port || url.username || url.password) {
		throw new Error(`Refusing unexpected Simplex origin: ${url.origin}`)
	}
	return url
}

function responseHeaders(headers: IncomingHttpHeaders): Headers {
	const output = new Headers()
	for (const [name, value] of Object.entries(headers)) {
		if (Array.isArray(value)) for (const item of value) output.append(name, item)
		else if (value !== undefined) output.set(name, value)
	}
	return output
}

/** Proxies one Chromium request to the daemon without changing its HTTP semantics. */
export function proxyToSimplex(
	request: Request,
	socketPath: string,
	requestImpl: HttpRequest = httpRequest,
	desktopVersion?: string,
): Promise<Response> {
	let url: URL
	try {
		url = simplexUrl(request)
	} catch (error) {
		return Promise.reject(error)
	}

	return new Promise((resolve, reject) => {
		let settled = false
		const options: RequestOptions = {
			socketPath,
			path: url.pathname + url.search,
			method: request.method,
			headers: Object.fromEntries(request.headers.entries()),
		}
		const proxied = requestImpl(options, (response) => {
			settled = true
			const status = response.statusCode ?? 502
			const noBody = status === 204 || status === 205 || status === 304
			if (noBody) response.resume()
			const headers = responseHeaders(response.headers)
			if (desktopVersion) headers.set("X-Simplex-Desktop-Version", desktopVersion)
			resolve(
				new Response(noBody ? null : (Readable.toWeb(response) as ReadableStream), {
					status,
					headers,
				}),
			)
		})
		const fail = (error: Error) => {
			if (!settled) reject(error)
		}
		proxied.once("error", fail)

		const abort = () =>
			proxied.destroy(Object.assign(new Error("Renderer request was aborted"), { code: "ABORT_ERR" }))
		if (request.signal.aborted) abort()
		else request.signal.addEventListener("abort", abort, { once: true })
		proxied.once("close", () => request.signal.removeEventListener("abort", abort))

		if (request.body) {
			const body = Readable.fromWeb(
				request.body as unknown as import("node:stream/web").ReadableStream<Uint8Array>,
			)
			body.once("error", (error) => proxied.destroy(error))
			body.pipe(proxied)
		} else {
			proxied.end()
		}
	})
}

async function regularFile(path: string): Promise<boolean> {
	try {
		return (await stat(path)).isFile()
	} catch {
		return false
	}
}

/** Serves the renderer shipped with Electron while keeping solver APIs on the private socket. */
export async function handleSimplexProtocol(
	request: Request,
	options: { socketPath: string; uiDistDir: string; desktopVersion?: string },
): Promise<Response> {
	const url = simplexUrl(request)
	if (url.pathname === "/health" || url.pathname.startsWith("/api/")) {
		return proxyToSimplex(request, options.socketPath, undefined, options.desktopVersion)
	}
	if (request.method !== "GET" && request.method !== "HEAD") {
		return new Response(JSON.stringify({ error: "Method not allowed" }), {
			status: 405,
			headers: { "Content-Type": "application/json", Allow: "GET, HEAD" },
		})
	}

	let decodedPath: string
	try {
		decodedPath = decodeURIComponent(url.pathname)
	} catch {
		return new Response("Bad request", { status: 400, headers: { "Content-Type": "text/plain" } })
	}
	if (decodedPath.includes("\0")) {
		return new Response("Bad request", { status: 400, headers: { "Content-Type": "text/plain" } })
	}
	const root = resolve(options.uiDistDir)
	const requested = resolve(root, `.${decodedPath === "/" ? "/index.html" : decodedPath}`)
	if (requested !== root && !requested.startsWith(`${root}${sep}`)) {
		return new Response("Forbidden", { status: 403, headers: { "Content-Type": "text/plain" } })
	}
	const target = (await regularFile(requested)) ? requested : resolve(root, "index.html")
	if (!(await regularFile(target))) {
		return new Response("Desktop UI not found", { status: 404, headers: { "Content-Type": "text/plain" } })
	}
	const immutableAsset = target === requested && requested.startsWith(`${root}${sep}assets${sep}`)
	return new Response(request.method === "HEAD" ? null : new Uint8Array(await readFile(target)), {
		status: 200,
		headers: {
			"Content-Type": MIME_TYPES[extname(target)] ?? "application/octet-stream",
			"Cache-Control": immutableAsset ? "public, max-age=31536000, immutable" : "no-cache",
		},
	})
}
