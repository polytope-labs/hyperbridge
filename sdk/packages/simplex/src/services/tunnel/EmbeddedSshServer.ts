import type { Socket } from "node:net"
import type { Duplex } from "node:stream"
import ssh2, { type Connection, type Server as SshServerType } from "ssh2"
import { getLogger } from "../Logger"
import { isLoopbackHost, markProvenance } from "../server/http-util"
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
	/**
	 * Hands an accepted channel to the UI server inside this process. Delivering
	 * it directly rather than dialling loopback means the UI can tell a tunnelled
	 * request from one the operator made at the keyboard, which is what keeps a
	 * paired device from managing remote access itself.
	 */
	deliver: (socket: Duplex, origin: Origin) => boolean
	/** Milliseconds a connection may spend unauthenticated. */
	authTimeoutMs?: number
	/** Failed attempts before the connection is dropped. */
	maxAuthFailures?: number
	/** Failed attempts from one source address inside `sourceWindowMs` before it is refused outright. */
	maxFailuresPerSource?: number
	sourceWindowMs?: number
}

/**
 * Ceiling on the failed-login table. Each entry is one source address, so a
 * scanner walking a /64 could otherwise add one forever. Eviction is
 * oldest-touched-first, which drops the addresses that stopped trying.
 */
const MAX_TRACKED_SOURCES = 10_000

/** How often the failed-login table is swept for entries that aged out. */
const FAILURE_SWEEP_INTERVAL_MS = 60_000

/**
 * Key exchange this server will accept.
 *
 * ssh2's default list includes diffie-hellman-group16/17/18-sha512, and it
 * negotiates by the *client's* preference order — so an unauthenticated peer
 * picks the algorithm. group18 costs ~107ms of synchronous CPU per handshake
 * on the event loop that also prices and fills orders, and a peer can force a
 * fresh one by renegotiating without ever attempting to log in. Curve25519 and
 * the ECDH groups are what every SSH app actually offers; group14 stays as the
 * floor for older ones, at ~2ms.
 */
const KEX_ALGORITHMS = [
	"curve25519-sha256@libssh.org",
	"curve25519-sha256",
	"ecdh-sha2-nistp256",
	"ecdh-sha2-nistp384",
	"ecdh-sha2-nistp521",
	"diffie-hellman-group14-sha256",
] as const

/**
 * Plain `zlib` starts compressing at NEWKEYS — before authentication — which
 * hands an unauthenticated peer a decompression amplifier. `zlib@openssh.com`
 * only starts once a session is authenticated, so it stays.
 */
const COMPRESSION_ALGORITHMS = ["none", "zlib@openssh.com"] as const

/**
 * How long a DISCONNECT is given to reach a well-behaved peer before the
 * stream is destroyed under it. Long enough for the reason to arrive, short
 * enough that a peer ignoring it holds nothing for long.
 */
const HANGUP_GRACE_MS = 250

/**
 * The deadline armed over one injected stream. ssh2 only tells us about a
 * connection once the peer has sent a complete SSH identification line, so a
 * peer that sends a partial line (or nothing) would otherwise never be seen,
 * never be timed out, and never be counted.
 */
interface StreamGuard {
	stream: Duplex
	origin: Origin
	timer?: NodeJS.Timeout
	authenticated: boolean
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
	/** Live authenticated connections per device fingerprint, so revoking one can hang up on it. */
	private readonly connectionsByDevice = new Map<string, Set<Connection>>()
	/** Streams awaiting authentication, so their deadlines can be disarmed together. */
	private readonly pending = new Set<StreamGuard>()
	/** Injected stream → its guard, and ssh2 connection → the same guard. */
	private readonly guardByStream = new WeakMap<Duplex, StreamGuard>()
	private readonly guardByConnection = new WeakMap<Connection, StreamGuard>()
	/** Set if ssh2 ever stops handing back the stream we injected; deadlines are then disarmed rather than risk reaping a live session. */
	private correlationBroken = false
	private lastFailureSweep = 0
	private live = 0

	constructor(private readonly opts: EmbeddedSshServerOptions) {
		this.authTimeoutMs = opts.authTimeoutMs ?? 30_000
		this.maxAuthFailures = opts.maxAuthFailures ?? 3
		this.maxFailuresPerSource = opts.maxFailuresPerSource ?? 10
		this.sourceWindowMs = opts.sourceWindowMs ?? 10 * 60 * 1000
		this.server = new SshServer(
			{
				hostKeys: [opts.hostKey],
				ident: "SSH-2.0-simplex",
				keepaliveInterval: 15_000,
				keepaliveCountMax: 3,
				algorithms: { kex: [...KEX_ALGORITHMS], compress: [...COMPRESSION_ALGORITHMS] },
			},
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
		// defineProperty, not assign: on a real net.Socket these are getter-only
		// and assigning throws. The relay always hands us an ssh2 channel today,
		// but inject() takes a Duplex and should not care which one.
		Object.defineProperties(socket, {
			remoteAddress: { value: origin.ip, configurable: true },
			remotePort: { value: origin.port, configurable: true },
			remoteFamily: { value: origin.ip.includes(":") ? "IPv6" : "IPv4", configurable: true },
		})
		this.arm(stream, origin)
		this.server.injectSocket(socket as Socket)
	}

	/**
	 * Puts a deadline on a stream that has not authenticated. ssh2 raises its
	 * connection event only after a complete identification line, so a peer that
	 * never sends one reaches no other timer in this class: it would sit there
	 * holding a stream, invisible to `connections`, for as long as it liked.
	 */
	private arm(stream: Duplex, origin: Origin): void {
		if (this.correlationBroken) return
		const guard: StreamGuard = { stream, origin, authenticated: false }
		guard.timer = setTimeout(() => {
			if (guard.authenticated) return
			this.logger.debug({ origin }, "Dropping tunnel connection: not authenticated in time")
			this.destroyQuietly(stream, origin)
		}, this.authTimeoutMs)
		guard.timer.unref?.()
		this.pending.add(guard)
		this.guardByStream.set(stream, guard)
		// Destroying a stream can raise 'error' on it; without a listener that is
		// an uncaught exception in the filler process.
		stream.on("error", (err) => this.logger.debug({ origin, err: err.message }, "Tunnel stream error"))
		stream.once("close", () => this.disarm(guard))
	}

	private disarm(guard: StreamGuard): void {
		if (guard.timer) clearTimeout(guard.timer)
		this.pending.delete(guard)
	}

	/**
	 * The stream ssh2 is running this connection over. ssh2 keeps the socket we
	 * injected on the connection; a miss means that internal changed, in which
	 * case every armed deadline is dropped — reaping nothing is a leak, reaping
	 * the wrong stream would cut a live operator session.
	 */
	private guardFor(conn: Connection): StreamGuard | undefined {
		const sock = (conn as unknown as { _sock?: Duplex })._sock
		const guard = sock ? this.guardByStream.get(sock) : undefined
		if (guard) {
			this.guardByConnection.set(conn, guard)
			return guard
		}
		if (!this.correlationBroken) {
			this.correlationBroken = true
			for (const pending of [...this.pending]) this.disarm(pending)
			this.logger.error(
				"Cannot match a tunnel connection to the stream it arrived on; unauthenticated connections will not be reaped",
			)
		}
		return undefined
	}

	/**
	 * Ends a connection and means it.
	 *
	 * `conn.end()` sends DISCONNECT and closes our write side only, so a peer
	 * that ignores it keeps the session, the protocol state and the connection
	 * count alive indefinitely. Destroying the stream is what reclaims them; the
	 * grace period is there so a well-behaved peer still gets the reason.
	 */
	private hangUp(conn: Connection): void {
		conn.end()
		const stream = this.guardByConnection.get(conn)?.stream
		if (!stream) return
		const origin = this.guardByConnection.get(conn)?.origin
		const timer = setTimeout(() => this.destroyQuietly(stream, origin), HANGUP_GRACE_MS)
		timer.unref?.()
	}

	/**
	 * Destroys a stream from a timer. Both callers run outside any request, so a
	 * stream whose `destroy()` throws synchronously would take the filler process
	 * with it — the one thing remote access must never do.
	 */
	private destroyQuietly(stream: Duplex, origin?: Origin): void {
		try {
			stream.destroy()
		} catch (err) {
			this.logger.debug({ origin, err }, "Tunnel stream refused to close")
		}
	}

	/**
	 * Hangs up on every live session holding this device key. Revoking a device
	 * blocks its next login, but a session opened a minute earlier would
	 * otherwise keep the dashboard open for as long as it stayed connected —
	 * and a lost phone is exactly when that matters.
	 */
	disconnectDevice(fingerprint: string): number {
		const open = this.connectionsByDevice.get(fingerprint)
		if (!open) return 0
		const count = open.size
		for (const conn of open) this.hangUp(conn)
		this.connectionsByDevice.delete(fingerprint)
		if (count > 0) this.logger.warn({ fingerprint, count }, "Closed live tunnel sessions for a revoked device")
		return count
	}

	close(): void {
		this.server.close()
	}

	private onConnection(conn: Connection, ip: string, port: number): void {
		this.live++
		const origin = `${ip}:${port}`
		const originIp = ip
		const originPort = port
		let failures = 0
		let authenticated = false
		/** The device key this connection authenticated with, once it has. */
		let deviceFingerprint: string | undefined
		// The deadline armed in inject() covers this connection too; resolving the
		// guard here is also what lets a hang-up destroy the stream.
		const guard = this.guardFor(conn)
		const authTimer = setTimeout(() => {
			if (!authenticated) {
				this.logger.debug({ origin }, "Dropping tunnel connection: not authenticated in time")
				this.hangUp(conn)
			}
		}, this.authTimeoutMs)

		conn.on("authentication", (ctx) => {
			const fail = (reason: string) => {
				failures++
				this.recordFailure(ip)
				this.logger.warn({ origin, reason, failures }, "Tunnel login refused")
				ctx.reject(["publickey"])
				if (failures >= this.maxAuthFailures) this.hangUp(conn)
			}
			// Every client opens with `none` to ask which methods the server wants;
			// refusing that is the handshake, not a failed login. Anything else —
			// password, keyboard-interactive, hostbased — is someone trying a door
			// that does not exist, and counts like any other refusal.
			if (ctx.method === "none") return ctx.reject(["publickey"])
			if (ctx.method !== "publickey") return fail(`unsupported authentication method ${ctx.method}`)
			const key = utils.parseKey(ctx.key.data)
			if (key instanceof Error) return fail("unreadable key")
			// The signature algorithm the client declared (`ctx.key.algo`, ssh2's
			// already-normalised form: both rsa-sha2-256 and rsa-sha2-512 arrive as
			// `ssh-rsa`) must match the key it actually presented. A mismatch is
			// key-algorithm confusion: offering an ed25519 key blob tagged as an RSA
			// signature algorithm parses as ed25519 here — so its fingerprint can
			// match an authorized device — while ssh2 drives verify() with a SHA-2
			// digest that key cannot compute. verify() then *throws*, which the
			// strict `!== true` below already rejects; this check refuses it earlier
			// and with a clear reason, and never rejects an honest client, whose
			// declared algorithm always matches its own key type.
			if (key.type !== ctx.key.algo) {
				return fail(`key type ${key.type} does not match offered algorithm ${ctx.key.algo}`)
			}
			const fingerprint = fingerprintOf(ctx.key.data)
			if (!this.opts.isAuthorized(fingerprint)) return fail(`unknown device key ${fingerprint}`)
			// A probe carries neither half: the client is asking whether this key
			// would be accepted, and saying yes only tells it to sign.
			if (ctx.signature === undefined && ctx.blob === undefined) return ctx.accept()
			// Anything else must reach the verification below. `ctx.accept()` is not
			// neutral: ssh2's PKAuthContext.accept() sends PK_OK only when there is no
			// signature and authenticates outright when there is one, so treating a
			// half-populated request as a probe would authenticate it unverified. ssh2
			// populates both fields together, which is the only reason the previous
			// `||` was not a bypass; refuse rather than keep resting on that.
			if (ctx.signature === undefined || ctx.blob === undefined) {
				return fail("malformed publickey request: a signature without the blob it signs, or the reverse")
			}
			// ssh2's key.verify() returns `true` on success and `false` on a normal
			// bad signature, but an *Error object* on a "more critical failure"
			// (e.g. an unsupported digest for the key). A loose `!key.verify(...)`
			// treats that Error as falsy and so lets it through — an auth bypass.
			// Require a strict boolean `true`.
			if (key.verify(ctx.blob, ctx.signature, ctx.hashAlgo) !== true) return fail("bad signature")
			authenticated = true
			if (guard) guard.authenticated = true
			deviceFingerprint = fingerprint
			const open = this.connectionsByDevice.get(fingerprint) ?? new Set<Connection>()
			open.add(conn)
			this.connectionsByDevice.set(fingerprint, open)
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
				// Authorization is re-read per channel, not just per login: a device
				// revoked mid-session opens nothing further, even before the hang-up
				// below lands.
				if (!deviceFingerprint || !this.opts.isAuthorized(deviceFingerprint)) {
					this.logger.warn({ origin, fingerprint: deviceFingerprint }, "Refused a forward for a revoked device")
					return reject()
				}
				const target = this.opts.target()
				if (!this.isTarget(info.destIP, info.destPort, target)) {
					this.logger.warn(
						{ origin, dest: `${info.destIP}:${info.destPort}` },
						"Refused a forward to a non-UI destination",
					)
					return reject()
				}
				const channel = accept()
				// The HTTP server reads a socket, not a bare stream: give it the
				// handful of methods it calls, the device's real origin for logs, and
				// the marker that says this request came in over the tunnel.
				Object.defineProperties(channel, {
					remoteAddress: { value: originIp, configurable: true },
					remotePort: { value: originPort, configurable: true },
					remoteFamily: { value: originIp.includes(":") ? "IPv6" : "IPv4", configurable: true },
				})
				// Stamped on a channel this process built, so a device cannot forge its
				// way out of the tunnel's reduced privileges.
				markProvenance(channel, "tunnel")
				Object.assign(channel, {
					setTimeout: () => channel,
					setNoDelay: () => channel,
					setKeepAlive: () => channel,
					ref: () => channel,
					unref: () => channel,
					destroySoon: () => channel.end(),
				})
				channel.on("error", (err: Error) => this.logger.debug({ origin, err: err.message }, "Tunnel channel error"))
				if (!this.opts.deliver(channel, { ip: originIp, port: originPort })) {
					// Not necessarily "no UI": the server also refuses a channel while it is
					// not listening, which says nothing about whether the dashboard exists.
					this.logger.warn({ origin }, "UI server would not accept the tunnelled connection")
					channel.destroy()
				}
			})
		})

		conn.on("error", (err) => this.logger.debug({ origin, err: err.message }, "Tunnel connection error"))
		conn.on("close", () => {
			clearTimeout(authTimer)
			this.live--
			if (deviceFingerprint) {
				const open = this.connectionsByDevice.get(deviceFingerprint)
				open?.delete(conn)
				if (open?.size === 0) this.connectionsByDevice.delete(deviceFingerprint)
			}
		})
	}

	/** The UI and only the UI: its port, on the address it is bound to or any loopback name for it. */
	private isTarget(host: string, port: number, target: { host: string; port: number }): boolean {
		if (port !== target.port) return false
		if (host === target.host) return true
		return isLoopbackHost(target.host) && isLoopbackHost(host)
	}

	private recordFailure(ip: string): void {
		const now = Date.now()
		if (now - this.lastFailureSweep >= FAILURE_SWEEP_INTERVAL_MS) {
			this.pruneFailures(now)
			this.lastFailureSweep = now
		}
		const hits = this.failuresBySource.get(ip) ?? []
		hits.push(now)
		// Delete before set so Map order stays oldest-touched first — that is what
		// makes the eviction below drop a stale address rather than an active one.
		this.failuresBySource.delete(ip)
		this.failuresBySource.set(ip, hits)
		if (this.failuresBySource.size > MAX_TRACKED_SOURCES) {
			const oldest = this.failuresBySource.keys().next().value
			if (oldest !== undefined && oldest !== ip) this.failuresBySource.delete(oldest)
		}
	}

	/** Drops sources whose failures have all aged out of the window. */
	private pruneFailures(now: number): void {
		const cutoff = now - this.sourceWindowMs
		for (const [ip, hits] of this.failuresBySource) {
			const kept = hits.filter((t) => t > cutoff)
			if (kept.length === 0) this.failuresBySource.delete(ip)
			else this.failuresBySource.set(ip, kept)
		}
	}

	private recentFailures(ip: string): number {
		const cutoff = Date.now() - this.sourceWindowMs
		const kept = (this.failuresBySource.get(ip) ?? []).filter((t) => t > cutoff)
		if (kept.length === 0) this.failuresBySource.delete(ip)
		else this.failuresBySource.set(ip, kept)
		return kept.length
	}
}
