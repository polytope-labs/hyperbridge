import { request as httpRequest, type IncomingHttpHeaders, type RequestOptions } from "node:http"
import { Readable } from "node:stream"

type HttpRequest = typeof httpRequest

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
): Promise<Response> {
	const url = new URL(request.url)
	if (url.protocol !== "simplex:" || url.hostname !== "local" || url.port || url.username || url.password) {
		return Promise.reject(new Error(`Refusing unexpected Simplex origin: ${url.origin}`))
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
			resolve(
				new Response(noBody ? null : (Readable.toWeb(response) as ReadableStream), {
					status,
					headers: responseHeaders(response.headers),
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
