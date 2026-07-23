import type { IncomingMessage, ServerResponse } from "node:http"

export const MAX_BODY_BYTES = 1_048_576

export function readBody(req: IncomingMessage): Promise<string> {
	return new Promise((resolve, reject) => {
		const chunks: Buffer[] = []
		let size = 0
		req.on("data", (chunk: Buffer) => {
			size += chunk.length
			if (size > MAX_BODY_BYTES) {
				reject(new Error("Request body too large"))
				req.destroy()
				return
			}
			chunks.push(chunk)
		})
		req.on("end", () => resolve(Buffer.concat(chunks).toString("utf-8")))
		req.on("error", reject)
	})
}

export function sendJson(res: ServerResponse, status: number, payload: unknown): void {
	res.writeHead(status, { "Content-Type": "application/json", "Cache-Control": "no-store" })
	res.end(JSON.stringify(payload))
}

export function isLoopbackHost(host: string): boolean {
	const normalized = host.toLowerCase()
	return normalized === "localhost" || normalized === "::1" || normalized.startsWith("127.")
}
