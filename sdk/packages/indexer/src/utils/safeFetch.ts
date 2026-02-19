import http from "http"
import https from "https"
import { URL } from "url"

interface SafeFetchOptions {
	method?: string
	headers?: Record<string, string>
	body?: string
}

interface SafeFetchResponse {
	ok: boolean
	status: number
	statusText: string
	headers: Record<string, string | string[] | undefined>
	json(): Promise<any>
	text(): Promise<string>
}

/**
 * A minimal VM2-safe fetch implementation using Node's built-in http/https modules.
 *
 * node-fetch v2.7.0 internally calls `process.version.substring(1)` which crashes
 * in the SubQuery VM2 sandbox where `process.version` is undefined and cannot be
 * polyfilled (the VM2 Proxy blocks writes to `process`).
 *
 * node-fetch v3.x is ESM-only and uses `URLSearchParams` which is also unavailable
 * in the VM2 sandbox.
 *
 * This wrapper provides a fetch-compatible interface for the simple JSON-RPC POST
 * requests used throughout the indexer, without depending on node-fetch.
 */
export function safeFetch(url: string, options: SafeFetchOptions = {}): Promise<SafeFetchResponse> {
	return new Promise((resolve, reject) => {
		const parsedUrl = new URL(url)

		const isHttps = parsedUrl.protocol === "https:"
		const transport = isHttps ? https : http

		const requestOptions: http.RequestOptions = {
			hostname: parsedUrl.hostname,
			port: parsedUrl.port || (isHttps ? 443 : 80),
			path: parsedUrl.pathname + parsedUrl.search,
			method: options.method || "GET",
			headers: options.headers || {},
		}

		const req = transport.request(requestOptions, (res) => {
			const chunks: Buffer[] = []

			res.on("data", (chunk: Buffer) => {
				chunks.push(chunk)
			})

			res.on("end", () => {
				const body = Buffer.concat(chunks).toString("utf8")
				const statusCode = res.statusCode || 0

				const response: SafeFetchResponse = {
					ok: statusCode >= 200 && statusCode < 300,
					status: statusCode,
					statusText: res.statusMessage || "",
					headers: res.headers as Record<string, string | string[] | undefined>,
					json() {
						return Promise.resolve(JSON.parse(body))
					},
					text() {
						return Promise.resolve(body)
					},
				}

				resolve(response)
			})

			res.on("error", reject)
		})

		req.on("error", reject)

		req.on("timeout", () => {
			req.destroy()
			reject(new Error(`Request to ${url} timed out`))
		})

		if (options.body) {
			req.write(options.body)
		}

		req.end()
	})
}