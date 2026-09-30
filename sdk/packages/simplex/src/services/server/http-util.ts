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

/**
 * How a connection reached the UI server. The three differ in who can open one,
 * which is what the request rules key off:
 *
 * - `tcp` — a listening port. Every local user can reach it, and a browser can
 *   be pointed at it, so it needs the host-header check below.
 * - `unix` — a Unix domain socket file, narrowed to the owning user. No browser
 *   can open one at all, so the host-header check is meaningless there.
 * - `tunnel` — a channel injected by the remote-access tunnel, already
 *   authenticated against a paired device key, and holding fewer privileges
 *   than a local caller (it cannot manage remote access).
 */
export type Provenance = "tcp" | "unix" | "tunnel"

/**
 * Symbol key carrying a socket's {@link Provenance}. A symbol, and only ever set
 * on sockets this process owns — the listener stamps what it accepted, the
 * tunnel stamps what it injected — so nothing a client sends can forge it.
 *
 * Generalises the older `VIA_TUNNEL` marker, which answered one question
 * (tunnelled or not) that a Unix socket made into three.
 */
export const PROVENANCE = Symbol.for("simplex.connection.provenance")

/**
 * Records how a socket arrived. The first stamp wins: the tunnel marks a channel
 * as it builds it, and the listener's blanket stamp must not overwrite that when
 * the channel is handed to the server.
 */
export function markProvenance(socket: object, provenance: Provenance): void {
	if (provenanceOf(socket) !== undefined) return
	Object.defineProperty(socket, PROVENANCE, { value: provenance, configurable: true })
}

/** How this request arrived, or undefined for a socket nothing stamped. */
export function provenanceOf(socket: unknown): Provenance | undefined {
	const tag = (socket as Record<symbol, unknown> | null | undefined)?.[PROVENANCE]
	return tag === "tcp" || tag === "unix" || tag === "tunnel" ? tag : undefined
}

/** Whether this request arrived through the remote-access tunnel. */
export function isTunnelled(socket: unknown): boolean {
	return provenanceOf(socket) === "tunnel"
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
