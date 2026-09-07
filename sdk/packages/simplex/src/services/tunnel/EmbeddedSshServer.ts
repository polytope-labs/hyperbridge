import { connect as tcpConnect, type Socket } from "node:net"
import type { Duplex } from "node:stream"
import ssh2, { type Connection, type Server as SshServerType } from "ssh2"
import { getLogger } from "../Logger"
import { isLoopbackHost } from "../server/http-util"
import { fingerprintOf } from "./keys"

// ssh2 is CommonJS: named imports resolve under vitest's transform but not in
// the ESM binary, where Node cannot see them statically.
const { Server: SshServer, utils } = ssh2

export interface EmbeddedSshServerOptions {
	/** OpenSSH-format private key; its fingerprint is what the phone pins. */
	hostKey: string
	/** Whether a device key (by SHA256 fingerprint) may open the UI. Consulted on every attempt, so revocation is immediate. */
	isAuthorized: (fingerprint: string) => boolean
	/** Where `direct-tcpip` channels may go: the UI server, and nothing else. Resolved per channel so a late bind still works. */
	target: () => { host: string; port: number }
	/** Milliseconds a connection may spend unauthenticated. */
	authTimeoutMs?: number
	/** Failed attempts before the connection is dropped. */
	maxAuthFailures?: number
	/** Failed attempts from one source address inside `sourceWindowMs` before it is refused outright. */
	maxFailuresPerSource?: number
	sourceWindowMs?: number
}

/** Where a forwarded connection originally came from, as the relay reports it. */
export interface Origin {
	ip: string
	port: number
}

/**
 * The SSH server the phone's session terminates in. It never listens on a
 * port: every connection arrives through the relay as a forwarded channel and
 * is injected here.
 *
 * Its whole job is to be a locked door with one keyhole. Public-key auth
 * against the paired device keys, no shells, exec, PTYs, subsystems, agent or
 * X11, and `direct-tcpip` only to the UI bind. Anyone who scans the relay can
 * reach this code, so it has to be that narrow.
 */
export class EmbeddedSshServer {
	private readonly server: SshServerType
	private readonly logger = getLogger("tunnel")
	private readonly authTimeoutMs: number
	private readonly maxAuthFailures: number
	private readonly maxFailuresPerSource: number
	private readonly sourceWindowMs: number
	private readonly failuresBySource = new Map<string, number[]>()
	private live = 0

	constructor(private readonly opts: EmbeddedSshServerOptions) {
		this.authTimeoutMs = opts.authTimeoutMs ?? 30_000
		this.maxAuthFailures = opts.maxAuthFailures ?? 3
		this.maxFailuresPerSource = opts.maxFailuresPerSource ?? 10
		this.sourceWindowMs = opts.sourceWindowMs ?? 10 * 60 * 1000
		this.server = new SshServer(
			{ hostKeys: [opts.hostKey], ident: "SSH-2.0-simplex", keepaliveInterval: 15_000, keepaliveCountMax: 3 },
			(conn, info) => this.onConnection(conn, info.ip, info.port),
		)
	}

	/** Connections currently open, authenticated or not. */
	get connections(): number {
		return this.live
	}

	/**
	 * Hands a forwarded stream to the SSH server as if it were an accepted TCP
	 * socket. Refused outright when the source has failed auth too often.
	 */
	inject(stream: Duplex, origin: Origin): void {
		if (this.recentFailures(origin.ip) >= this.maxFailuresPerSource) {
			this.logger.warn({ origin }, "Refusing tunnel connection: too many failed logins from this source")
			stream.destroy()
			return
		}
		const socket = stream as Duplex & Partial<Socket>
		Object.assign(socket, {
			remoteAddress: origin.ip,
			remotePort: origin.port,
			remoteFamily: origin.ip.includes(":") ? "IPv6" : "IPv4",
		})
		this.server.injectSocket(socket as Socket)
	}

	close(): void {
		this.server.close()
	}

	private onConnection(conn: Connection, ip: string, port: number): void {
		this.live++
		const origin = `${ip}:${port}`
		let failures = 0
		let authenticated = false
		const authTimer = setTimeout(() => {
			if (!authenticated) {
				this.logger.debug({ origin }, "Dropping tunnel connection: not authenticated in time")
				conn.end()
			}
		}, this.authTimeoutMs)

		conn.on("authentication", (ctx) => {
			const fail = (reason: string) => {
				failures++
				this.recordFailure(ip)
				this.logger.warn({ origin, reason, failures }, "Tunnel login refused")
				ctx.reject(["publickey"])
				if (failures >= this.maxAuthFailures) conn.end()
			}
			if (ctx.method !== "publickey") return ctx.reject(["publickey"])
			const key = utils.parseKey(ctx.key.data)
			if (key instanceof Error) return fail("unreadable key")
			const fingerprint = fingerprintOf(ctx.key.data)
			if (!this.opts.isAuthorized(fingerprint)) return fail(`unknown device key ${fingerprint}`)
			// No signature yet: the client is asking whether this key would be
			// accepted. Saying yes only tells it to sign.
			if (ctx.signature === undefined || ctx.blob === undefined) return ctx.accept()
			if (!key.verify(ctx.blob, ctx.signature, ctx.hashAlgo)) return fail("bad signature")
			authenticated = true
			this.logger.info({ origin, fingerprint }, "Device connected through the tunnel")
			ctx.accept()
		})

		conn.on("ready", () => {
			clearTimeout(authTimer)
			conn.on("session", (_accept, reject) => {
				this.logger.warn({ origin }, "Refused a session channel (no shell through the tunnel)")
				reject()
			})
			conn.on("openssh.streamlocal", (_accept, reject) => reject())
			conn.on("request", (_accept, reject) => reject?.())
			conn.on("tcpip", (accept, reject, info) => {
				const target = this.opts.target()
				if (!this.isTarget(info.destIP, info.destPort, target)) {
					this.logger.warn(
						{ origin, dest: `${info.destIP}:${info.destPort}` },
						"Refused a forward to a non-UI destination",
					)
					return reject()
				}
				const channel = accept()
				const upstream = tcpConnect(target.port, target.host)
				const teardown = () => {
					upstream.destroy()
					channel.destroy()
				}
				upstream.once("connect", () => {
					upstream.pipe(channel).pipe(upstream)
				})
				upstream.on("error", (err) => {
					this.logger.warn({ origin, err }, "UI connection failed behind the tunnel")
					teardown()
				})
				upstream.on("close", teardown)
				channel.on("close", teardown)
				channel.on("error", teardown)
			})
		})

		conn.on("error", (err) => this.logger.debug({ origin, err: err.message }, "Tunnel connection error"))
		conn.on("close", () => {
			clearTimeout(authTimer)
			this.live--
		})
	}

	/** The UI and only the UI: its port, on the address it is bound to or any loopback name for it. */
	private isTarget(host: string, port: number, target: { host: string; port: number }): boolean {
		if (port !== target.port) return false
		if (host === target.host) return true
		return isLoopbackHost(target.host) && isLoopbackHost(host)
	}

	private recordFailure(ip: string): void {
		const hits = this.failuresBySource.get(ip) ?? []
		hits.push(Date.now())
		this.failuresBySource.set(ip, hits)
	}

	private recentFailures(ip: string): number {
		const cutoff = Date.now() - this.sourceWindowMs
		const kept = (this.failuresBySource.get(ip) ?? []).filter((t) => t > cutoff)
		if (kept.length === 0) this.failuresBySource.delete(ip)
		else this.failuresBySource.set(ip, kept)
		return kept.length
	}
}
