import { existsSync } from "node:fs"
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

/**
 * A container's network namespace is its own boundary: 0.0.0.0 inside one is not the host's
 * 0.0.0.0, and what actually reaches the machine is whatever the operator published with
 * `-p`. Loopback inside a container is unreachable from the host entirely — Docker Desktop
 * on macOS and Windows runs the daemon in a VM with no host networking — so treating a
 * wildcard bind as remote exposure there blocks the setup path instead of protecting it.
 *
 * Both files are created by the runtime (Docker, Podman) outside the image, so a workload
 * cannot forge its way past the loopback rule by shipping one.
 */
export function isContainerized(): boolean {
	return existsSync("/.dockerenv") || existsSync("/run/.containerenv")
}

/**
 * DNS-rebinding defense: a rebound attacker origin always presents a DNS name
 * in the Host header, so only IP literals (and localhost) are accepted. When
 * the server is bound to loopback, the Host must itself be loopback.
 */
export function hostHeaderAllowed(hostHeader: string | undefined, boundLoopback: boolean): boolean {
	if (!hostHeader) return false
	// Strip the port: "[::1]:8686" and "127.0.0.1:8686" both carry one.
	const bracketed = hostHeader.match(/^\[([^\]]+)\](?::\d+)?$/)
	const hostname = (bracketed ? bracketed[1] : hostHeader.replace(/:\d+$/, "")).toLowerCase()
	if (boundLoopback) return isLoopbackHost(hostname)
	const isIpv4 = /^\d{1,3}(\.\d{1,3}){3}$/.test(hostname)
	const isIpv6 = hostname.includes(":")
	return hostname === "localhost" || isIpv4 || isIpv6
}
