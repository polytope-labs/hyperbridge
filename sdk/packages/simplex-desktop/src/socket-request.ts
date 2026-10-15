import { request as httpRequest } from "node:http"

const MAX_RESPONSE_BYTES = 64 * 1024

/** Bounded control requests only; the renderer's streaming proxy has a separate lifetime. */
export function socketRequest(
	socketPath: string,
	path: string,
	method: "GET" | "POST",
	timeoutMs = 1_000,
): Promise<{ status: number; body: string }> {
	return new Promise((resolve, reject) => {
		let settled = false
		const request = httpRequest({
			socketPath,
			path,
			method,
			headers: method === "POST" ? { "X-Simplex-UI": "1" } : undefined,
		})
		const fail = (error: Error) => {
			if (settled) return
			settled = true
			clearTimeout(timer)
			reject(error)
			request.destroy()
		}
		// An absolute deadline also covers a listener that keeps sending partial data.
		const timer = setTimeout(
			() => fail(Object.assign(new Error(`Simplex ${path} request timed out`), { code: "ETIMEDOUT" })),
			timeoutMs,
		)
		const interrupted = () => fail(new Error(`Simplex ${path} response ended before completion`))
		request.once("error", fail)
		request.once("close", () => {
			if (!settled) interrupted()
		})
		request.once("response", (response) => {
			const chunks: Buffer[] = []
			let bytes = 0
			response.once("error", fail)
			response.once("aborted", interrupted)
			response.once("close", () => {
				if (!response.complete) interrupted()
			})
			response.on("data", (chunk: Buffer) => {
				if (settled) return
				bytes += chunk.length
				if (bytes > MAX_RESPONSE_BYTES) {
					fail(new Error(`Simplex ${path} response exceeded ${MAX_RESPONSE_BYTES} bytes`))
					return
				}
				chunks.push(chunk)
			})
			response.once("end", () => {
				if (settled) return
				settled = true
				clearTimeout(timer)
				resolve({ status: response.statusCode ?? 0, body: Buffer.concat(chunks).toString("utf8") })
			})
		})
		request.end()
	})
}
