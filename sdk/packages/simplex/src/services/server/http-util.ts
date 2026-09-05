import { existsSync } from "node:fs"
import type { IncomingMessage, ServerResponse } from "node:http"
import { isIP } from "node:net"

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
	if (normalized === "localhost") return true
	// IPv6 loopback, plus the IPv4-mapped form some stacks present.
	if (normalized === "::1" || normalized === "::ffff:127.0.0.1") return true
	// Only a genuine IPv4 literal in 127.0.0.0/8 counts. A prefix test on the raw
	// string would also match DNS names like "127.0.0.1.evil.com" (a leading-digit
	// label is a legal hostname), which is exactly the DNS-rebinding bypass — so the
	// host must first parse as an IPv4 address before its first octet is trusted.
	if (isIP(normalized) === 4) return normalized.split(".")[0] === "127"
	return false
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
	// A rebound attacker origin always presents a DNS name; only IP literals (and the
	// literal "localhost") are accepted. `isIP` rejects DNS names and a stray non-numeric
	// port that survived the strip (e.g. "evil.com:abc"), which a bare `includes(":")`
	// IPv6 test would have waved through.
	return hostname === "localhost" || isIP(hostname) !== 0
}
