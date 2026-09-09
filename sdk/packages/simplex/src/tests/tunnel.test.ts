import { describe, it, expect, afterEach, beforeAll, vi } from "vitest"
import { createServer as createHttpServer, type Server as HttpServer } from "node:http"
import { createServer as createTcpServer, type Server as TcpServer } from "node:net"
import { randomBytes } from "node:crypto"
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs"
import { Duplex, PassThrough } from "node:stream"
import { tmpdir } from "node:os"
import { join } from "node:path"
import ssh2, {
	type Client as SshClientType,
	type ClientChannel,
	type Connection,
	type Server as SshServerType,
} from "ssh2"
import {
	TunnelService,
	parseRelayAddress,
	expectedRelayFingerprint,
	relayKey,
	DEFAULT_TUNNEL_RELAY,
	DEFAULT_TUNNEL_RELAY_HOST_KEY,
} from "@/services/tunnel/TunnelService"
import { EmbeddedSshServer } from "@/services/tunnel/EmbeddedSshServer"
import { isTunnelled } from "@/services/server/http-util"
import {
	TunnelKeyStore,
	fingerprintOf,
	fingerprintOfKeyText,
	generateKeyPair,
	normalizePublicKey,
} from "@/services/tunnel/keys"

const { Client: SshClient, Server: SshServer, utils } = ssh2

/**
 * A stand-in for simplex-tunnel, the Rust relay: accepts any public key,
 * opens a loopback port on `tcpip-forward`, and pipes every connection on it
 * back to the operator as a `forwarded-tcpip` channel with the requested bind
 * address echoed verbatim (which is what ssh2 matches channels on).
 */
class FakeRelay {
	readonly hostKey = generateKeyPair()
	readonly hostFingerprint = fingerprintOfKeyText(this.hostKey.public)
	private server: SshServerType
	private listeners = new Set<TcpServer>()
	private connections = new Set<Connection>()
	port = 0
	/** Leased port of the most recent forward request. */
	leased?: number
	sessions = 0

	constructor(private readonly acceptForward = true) {
		this.server = new SshServer({ hostKeys: [this.hostKey.private] }, (conn) => this.onConnection(conn))
	}

	start(): Promise<number> {
		return new Promise((resolve) => {
			this.server.listen(0, "127.0.0.1", () => {
				this.port = (this.server.address() as { port: number }).port
				resolve(this.port)
			})
		})
	}

	/** Stops listening and drops every operator session, as a crashed relay would. */
	stop(): void {
		for (const l of this.listeners) l.close()
		for (const conn of this.connections) conn.end()
		this.server.close()
	}

	private onConnection(conn: Connection): void {
		this.sessions++
		this.connections.add(conn)
		// A client that rejects our host key surfaces here as KEY_EXCHANGE_FAILED.
		conn.on("error", () => {})
		conn.on("authentication", (ctx) => (ctx.method === "publickey" ? ctx.accept() : ctx.reject(["publickey"])))
		conn.on("request", (accept, reject, name, info) => {
			if (name !== "tcpip-forward" || !this.acceptForward) return reject?.()
			const { bindAddr } = info as { bindAddr: string; bindPort: number }
			const listener = createTcpServer((sock) => {
				const port = (listener.address() as { port: number }).port
				conn.forwardOut(
					bindAddr,
					port,
					sock.remoteAddress ?? "127.0.0.1",
					sock.remotePort ?? 0,
					(err, channel) => {
						if (err) return sock.destroy()
						sock.pipe(channel).pipe(sock)
						// A rejected handshake (e.g. the kex test) tears one side down
						// while the other is still writing; the resulting EPIPE has no
						// listener and would surface as an unhandled error attributed to
						// whichever test happens to be running. A real relay never crashes
						// on a client hang-up either.
						channel.on("error", () => {})
						sock.on("error", () => {})
						channel.on("close", () => sock.destroy())
						sock.on("close", () => channel.destroy())
					},
				)
			})
			this.listeners.add(listener)
			listener.listen(0, "127.0.0.1", () => {
				this.leased = (listener.address() as { port: number }).port
				accept?.(this.leased)
			})
		})
		conn.on("close", () => {
			this.sessions--
			this.connections.delete(conn)
		})
	}
}

function startUi(): Promise<{ server: HttpServer; port: number }> {
	return new Promise((resolve) => {
		const server = createHttpServer((req, res) => {
			res.setHeader("Content-Type", "text/plain")
			res.end(`ui ok host=${req.headers.host}`)
		})
		server.listen(0, "127.0.0.1", () => resolve({ server, port: (server.address() as { port: number }).port }))
	})
}

async function waitFor(check: () => boolean, ms = 8000): Promise<void> {
	const deadline = Date.now() + ms
	while (!check()) {
		if (Date.now() > deadline) throw new Error("timed out waiting")
		await new Promise((r) => setTimeout(r, 25))
	}
}

/** A phone: an ssh2 client holding a device key. */
function phoneConnect(opts: {
	port: number
	privateKey: string
	expectHostFingerprint?: string
}): Promise<SshClientType> {
	return new Promise((resolve, reject) => {
		const client = new SshClient()
		client.on("ready", () => resolve(client))
		client.on("error", reject)
		client.connect({
			host: "127.0.0.1",
			port: opts.port,
			username: "simplex",
			privateKey: opts.privateKey,
			readyTimeout: 5000,
			hostVerifier: (key: Buffer) =>
				opts.expectHostFingerprint === undefined || fingerprintOf(key) === opts.expectHostFingerprint,
		})
	})
}

/** An in-memory stand-in for a relay-forwarded channel, which is what inject() really receives. */
function pipePair(): { clientSide: Duplex; serverSide: Duplex } {
	const a2b = new PassThrough()
	const b2a = new PassThrough()
	const clientSide = Duplex.from({ readable: b2a, writable: a2b })
	const serverSide = Duplex.from({ readable: a2b, writable: b2a })
	// Destroying a Duplex.from composite aborts the streams underneath it, and an
	// unheard 'error' there fails the run. A real relay channel is one stream.
	for (const stream of [a2b, b2a, clientSide, serverSide]) stream.on("error", () => {})
	return { clientSide, serverSide }
}

/** A peer that ignores the server's polite close, the way a hostile one would. */
function stubborn(stream: Duplex): never {
	const proxy: unknown = new Proxy(stream, {
		get(target, key, receiver) {
			if (key === "end") return () => proxy
			const value = Reflect.get(target, key, receiver)
			return typeof value === "function" ? value.bind(target) : value
		},
	})
	return proxy as never
}

/** Hands a stream to the service's embedded server, as the relay client does. */
function injectInto(service: TunnelService, stream: Duplex, origin: { ip: string; port: number }): void {
	;(service as unknown as { server: EmbeddedSshServer }).server.inject(stream, origin)
}

/** Sends one raw HTTP request through a direct-tcpip channel and returns the response text. */
function httpThrough(
	client: SshClientType,
	host: string,
	port: number,
	request: { method?: string; path?: string } = {},
): Promise<string> {
	const method = request.method ?? "GET"
	const path = request.path ?? "/health"
	return new Promise((resolve, reject) => {
		client.forwardOut("127.0.0.1", 0, host, port, (err: Error | undefined, stream: ClientChannel) => {
			if (err) return reject(err)
			let body = ""
			stream.on("data", (chunk: Buffer) => {
				body += chunk.toString()
			})
			stream.on("close", () => resolve(body))
			stream.on("error", reject)
			// The UI requires this header on every mutation; a device sets it as
			// easily as the operator's browser does, which is the point.
			const headers = method === "GET" || method === "HEAD" ? "" : "X-Simplex-UI: 1\r\nContent-Length: 2\r\n"
			const payload = method === "GET" || method === "HEAD" ? "" : "{}"
			stream.end(
				`${method} ${path} HTTP/1.1\r\nHost: localhost:8686\r\n${headers}Connection: close\r\n\r\n${payload}`,
			)
		})
	})
}

describe("relayKey", () => {
	it("treats every spelling of one relay as the same relay", () => {
		// The hosted relay's pin used to be keyed on the exact default string, so
		// writing it without the port — which the config explicitly allows —
		// skipped the shipped pin and trusted whatever answered first.
		expect(relayKey("simplex.tunnel.polytope.technology")).toBe(relayKey(DEFAULT_TUNNEL_RELAY))
		expect(relayKey("SIMPLEX.Tunnel.Polytope.Technology:443")).toBe(relayKey(DEFAULT_TUNNEL_RELAY))
		expect(relayKey("simplex.tunnel.polytope.technology.:443")).toBe(relayKey(DEFAULT_TUNNEL_RELAY))
		expect(relayKey("other.example:443")).not.toBe(relayKey(DEFAULT_TUNNEL_RELAY))
		expect(relayKey("other.example:2222")).not.toBe(relayKey("other.example"))
	})

	it("applies the built-in pin to the port-less spelling of the hosted relay", () => {
		expect(expectedRelayFingerprint({}, "simplex.tunnel.polytope.technology", undefined)).toBe(
			DEFAULT_TUNNEL_RELAY_HOST_KEY,
		)
		expect(expectedRelayFingerprint({}, DEFAULT_TUNNEL_RELAY, undefined)).toBe(DEFAULT_TUNNEL_RELAY_HOST_KEY)
	})
})

describe("parseRelayAddress", () => {
	it("defaults the port to 443 and understands IPv6 brackets", () => {
		expect(parseRelayAddress("relay.example.com")).toEqual({ host: "relay.example.com", port: 443 })
		expect(parseRelayAddress("relay.example.com:2222")).toEqual({ host: "relay.example.com", port: 2222 })
		expect(parseRelayAddress("[::1]:443")).toEqual({ host: "::1", port: 443 })
		expect(parseRelayAddress("::1")).toEqual({ host: "::1", port: 443 })
		expect(parseRelayAddress(DEFAULT_TUNNEL_RELAY)).toEqual({
			host: "simplex.tunnel.polytope.technology",
			port: 443,
		})
		expect(() => parseRelayAddress("")).toThrow(/required/)
		expect(() => parseRelayAddress("host:99999")).toThrow(/out of range/)
		expect(() => parseRelayAddress("user@host")).toThrow(/host\[:port\]/)
	})
})

describe("expectedRelayFingerprint", () => {
	it("pins the hosted relay by default, lets config override, and falls back to first contact", () => {
		const known = { relay: "other:443", fingerprint: "SHA256:known" }
		expect(expectedRelayFingerprint({}, DEFAULT_TUNNEL_RELAY, undefined)).toBe(DEFAULT_TUNNEL_RELAY_HOST_KEY)
		expect(expectedRelayFingerprint({}, DEFAULT_TUNNEL_RELAY, known)).toBe(DEFAULT_TUNNEL_RELAY_HOST_KEY)
		expect(expectedRelayFingerprint({ relayHostKey: " SHA256:mine " }, DEFAULT_TUNNEL_RELAY, undefined)).toBe(
			"SHA256:mine",
		)
		expect(expectedRelayFingerprint({}, "other:443", known)).toBe("SHA256:known")
		expect(expectedRelayFingerprint({}, "other:443", undefined)).toBeUndefined()
		expect(expectedRelayFingerprint({}, "third:443", known)).toBeUndefined()
	})
})

describe("TunnelKeyStore", () => {
	it("creates keys once, pairs and revokes devices, and pins the relay", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-tunnel-keys-"))
		const store = new TunnelKeyStore(dir)
		const operator = store.operatorKey()
		expect(operator.fingerprint).toMatch(/^SHA256:/)
		expect(new TunnelKeyStore(dir).operatorKey().fingerprint).toBe(operator.fingerprint)
		expect(store.hostKey().fingerprint).not.toBe(operator.fingerprint)

		const { device, privateKey } = store.addDevice("Seun's phone")
		expect(privateKey).toContain("OPENSSH PRIVATE KEY")
		expect(fingerprintOfKeyText(privateKey!)).toBe(device.fingerprint)
		expect(store.isAuthorized(device.fingerprint)).toBe(true)
		expect(store.devices()).toEqual([device])
		// The file is plain authorized_keys: one key per line, label in the comment.
		expect(readFileSync(join(dir, "tunnel", "authorized_keys"), "utf8")).toMatch(
			/^ssh-ed25519 \S+ simplex-device:Seun's%20phone:\d+\n$/,
		)
		expect(() => store.addDevice("   ")).toThrow(/label/)

		expect(store.removeDevice(device.fingerprint)).toBe(true)
		expect(store.removeDevice(device.fingerprint)).toBe(false)
		expect(store.isAuthorized(device.fingerprint)).toBe(false)

		expect(store.knownRelay("relay:443")).toBeUndefined()
		store.rememberRelay("relay:443", "SHA256:abc")
		expect(store.knownRelay("relay:443")).toEqual({ relay: "relay:443", fingerprint: "SHA256:abc" })
	})

	it("pins each relay address separately, so moving away and back keeps the old pin", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-tunnel-pins-"))
		const store = new TunnelKeyStore(dir)
		store.rememberRelay("a:443", "SHA256:aaa")
		store.rememberRelay("b:443", "SHA256:bbb")
		// The second relay does not evict the first: coming back to A still has a
		// pin to check its key against, instead of trusting whatever it presents.
		expect(store.knownRelay("a:443")).toEqual({ relay: "a:443", fingerprint: "SHA256:aaa" })
		expect(store.knownRelay("b:443")).toEqual({ relay: "b:443", fingerprint: "SHA256:bbb" })
		expect(store.knownRelay("c:443")).toBeUndefined()
		// Re-pinning one address rewrites that line only.
		store.rememberRelay("a:443", "SHA256:zzz")
		expect(store.knownRelay("a:443")).toEqual({ relay: "a:443", fingerprint: "SHA256:zzz" })
		expect(store.knownRelay("b:443")).toEqual({ relay: "b:443", fingerprint: "SHA256:bbb" })
		expect(readFileSync(join(dir, "tunnel", "known_relay"), "utf8").trim().split("\n")).toHaveLength(2)
	})

	it("reads a pre-per-relay known_relay file written as a single line", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-tunnel-legacy-pin-"))
		mkdirSync(join(dir, "tunnel"), { recursive: true })
		writeFileSync(join(dir, "tunnel", "known_relay"), "old:443 SHA256:old\n", { mode: 0o600 })
		expect(new TunnelKeyStore(dir).knownRelay("old:443")).toEqual({ relay: "old:443", fingerprint: "SHA256:old" })
	})

	it("throws away a key pair ssh2 generates but cannot read back", () => {
		// ssh2 emits an unparseable pair roughly once in 256. A stored key that
		// cannot be read back breaks remote access on every subsequent boot, so
		// generation retries rather than persisting the bad one.
		// A pair known to parse, taken before the spy is installed — asking ssh2 for
		// a fresh one inside the mock would reintroduce the very 1-in-256 flake
		// this test exists to cover.
		const usable = generateKeyPair("usable")
		const broken = {
			public: "ssh-ed25519 not-a-key",
			private: "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----\n",
		}
		let calls = 0
		const spy = vi.spyOn(ssh2.utils, "generateKeyPairSync").mockImplementation((() => {
			calls++
			return calls <= 2 ? broken : usable
		}) as typeof ssh2.utils.generateKeyPairSync)
		try {
			const pair = generateKeyPair("retry me")
			expect(calls).toBe(3)
			expect(ssh2.utils.parseKey(pair.private)).not.toBeInstanceOf(Error)
			// Nothing usable at all is an error, not an infinite loop.
			calls = 0
			spy.mockImplementation((() => {
				calls++
				return broken
			}) as typeof ssh2.utils.generateKeyPairSync)
			expect(() => generateKeyPair("never works")).toThrow(/Could not generate/)
			expect(calls).toBe(8)
		} finally {
			spy.mockRestore()
		}
	})

	it("pairs a pasted public key without ever holding the private half", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-tunnel-paste-"))
		const store = new TunnelKeyStore(dir)
		const own = utils.generateKeyPairSync("ed25519", { comment: "from the phone app" })
		const { device, privateKey } = store.addDevice("own key", `  ${own.public}\n`)
		expect(privateKey).toBeUndefined()
		expect(device.publicKey).toBe(normalizePublicKey(own.public))
		expect(device.publicKey).not.toContain("from the phone app")
		expect(fingerprintOfKeyText(own.private)).toBe(device.fingerprint)
		expect(store.isAuthorized(device.fingerprint)).toBe(true)
		// The same key cannot be paired twice, and private keys are refused.
		expect(() => store.addDevice("again", own.public)).toThrow(/already paired/)
		expect(() => store.addDevice("oops", own.private)).toThrow(/private key/)
		expect(() => store.addDevice("junk", "ssh-ed25519 notbase64")).toThrow(/Not a valid SSH public key/)
		expect(() => store.addDevice("empty", "   ")).toThrow(/Paste the device/)
		const rsa = utils.generateKeyPairSync("rsa", { bits: 2048 })
		expect(store.addDevice("rsa", rsa.public).device.publicKey).toMatch(/^ssh-rsa /)
	})
})

// Real SSH handshakes over loopback: a few seconds normally, longer on a cold
// or loaded runner, so the default 5s budget is not enough.
describe("TunnelService", { timeout: 30_000 }, () => {
	let relay: FakeRelay
	let ui: { server: HttpServer; port: number }
	let tunnel: TunnelService | undefined
	const phones: SshClientType[] = []

	beforeAll(async () => {
		ui = await startUi()
	})

	afterEach(async () => {
		for (const phone of phones.splice(0)) phone.end()
		await tunnel?.stop()
		tunnel = undefined
		relay?.stop()
	})

	function startTunnel(overrides: { enabled?: boolean; relayHostKey?: string; relay?: string } = {}): TunnelService {
		const dir = mkdtempSync(join(tmpdir(), "simplex-tunnel-"))
		tunnel = new TunnelService({
			dataDir: dir,
			config: { enabled: true, relay: `127.0.0.1:${relay.port}`, ...overrides },
			uiTarget: () => ({ host: "127.0.0.1", port: ui.port }),
			// The real binary hands the channel to its UI server in-process; the
			// test's UI server takes it the same way.
			deliver: (socket) => {
				ui.server.emit("connection", socket)
				return true
			},
			maxBackoffMs: 200,
		})
		tunnel.start()
		return tunnel
	}

	it("stays off unless enabled", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel({ enabled: false })
		expect(service.status().state).toBe("disabled")
		expect(service.status().relay).toBe(`127.0.0.1:${relay.port}`)
		expect(
			new TunnelService({
				dataDir: mkdtempSync(join(tmpdir(), "x-")),
				uiTarget: () => ({ host: "127.0.0.1", port: 1 }),
			}).status().relay,
		).toBe(DEFAULT_TUNNEL_RELAY)
		expect(relay.sessions).toBe(0)
	})

	it("connects, leases a port, and lets a paired phone reach the UI and nothing else", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel()
		await waitFor(() => service.status().state === "connected")
		const status = service.status()
		expect(status.port).toBe(relay.leased)
		expect(status.relayFingerprint).toBe(relay.hostFingerprint)

		const paired = service.addDevice("phone")
		expect(paired.connection).toMatchObject({
			host: "127.0.0.1",
			port: relay.leased,
			username: "simplex",
			hostFingerprint: status.hostFingerprint,
			localForward: `8686:127.0.0.1:${ui.port}`,
		})

		const phone = await phoneConnect({
			port: status.port!,
			privateKey: paired.privateKey!,
			expectHostFingerprint: status.hostFingerprint,
		})
		phones.push(phone)
		const response = await httpThrough(phone, "127.0.0.1", ui.port)
		expect(response).toContain("ui ok host=localhost:8686")
		// "localhost" is the same UI; any other port or host is not.
		expect(await httpThrough(phone, "localhost", ui.port)).toContain("ui ok")
		await expect(httpThrough(phone, "127.0.0.1", ui.port + 1)).rejects.toThrow()
		await expect(httpThrough(phone, "example.com", 80)).rejects.toThrow()
		await expect(
			new Promise((resolve, reject) =>
				phone.exec("id", (err: Error | undefined) => (err ? reject(err) : resolve(undefined))),
			),
		).rejects.toThrow()

		// A phone that generated its own key and pasted the public half works the same way.
		const own = utils.generateKeyPairSync("ed25519")
		const pasted = service.addDevice("own", own.public)
		expect(pasted.privateKey).toBeUndefined()
		const ownPhone = await phoneConnect({
			port: status.port!,
			privateKey: own.private,
			expectHostFingerprint: status.hostFingerprint,
		})
		phones.push(ownPhone)
		expect(await httpThrough(ownPhone, "127.0.0.1", ui.port)).toContain("ui ok")

		// A revoked device cannot come back.
		expect(service.removeDevice(paired.device.fingerprint)).toBe(true)
		await expect(phoneConnect({ port: status.port!, privateKey: paired.privateKey! })).rejects.toThrow()
		// Neither can a key simplex never issued.
		const stranger = utils.generateKeyPairSync("ed25519")
		await expect(phoneConnect({ port: status.port!, privateKey: stranger.private })).rejects.toThrow()
	})

	it("refuses a relay whose host key does not match the pin", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel({ relayHostKey: "SHA256:notTheRelay" })
		await waitFor(() => service.status().lastError !== undefined)
		expect(service.status().lastError).toMatch(/host key mismatch/)
		expect(service.status().state).not.toBe("connected")
	})

	it("pins the relay key on first contact and remembers it", async () => {
		relay = new FakeRelay()
		await relay.start()
		const dir = mkdtempSync(join(tmpdir(), "simplex-tunnel-pin-"))
		const opts = { dataDir: dir, uiTarget: () => ({ host: "127.0.0.1", port: ui.port }), maxBackoffMs: 200 }
		tunnel = new TunnelService({ ...opts, config: { enabled: true, relay: `127.0.0.1:${relay.port}` } })
		tunnel.start()
		await waitFor(() => tunnel!.status().state === "connected")
		expect(new TunnelKeyStore(dir).knownRelay(`127.0.0.1:${relay.port}`)).toEqual({
			relay: `127.0.0.1:${relay.port}`,
			fingerprint: relay.hostFingerprint,
		})
		await tunnel.stop()

		// Same address, different key: refused.
		const impostor = new FakeRelay()
		const port = await impostor.start()
		tunnel = new TunnelService({ ...opts, config: { enabled: true, relay: `127.0.0.1:${relay.port}` } })
		// Point the stored pin at the impostor's port so the address matches.
		new TunnelKeyStore(dir).rememberRelay(`127.0.0.1:${port}`, relay.hostFingerprint)
		await tunnel.configure({ relay: `127.0.0.1:${port}` })
		await waitFor(() => tunnel!.status().lastError !== undefined)
		expect(tunnel.status().lastError).toMatch(/host key mismatch/)
		impostor.stop()
	})

	it("reconnects after the relay drops it and reports the outage meanwhile", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel()
		await waitFor(() => service.status().state === "connected")
		relay.stop()
		await waitFor(() => service.status().state === "reconnecting")
		expect(service.status().port).toBeUndefined()

		relay = new FakeRelay()
		const port = await relay.start()
		await service.configure({ relay: `127.0.0.1:${port}` })
		await waitFor(() => service.status().state === "connected")
		expect(service.status().port).toBe(relay.leased)
	})

	it("can be turned off and on from configure", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel()
		await waitFor(() => service.status().state === "connected")
		await service.configure({ enabled: false })
		expect(service.status()).toMatchObject({ enabled: false, state: "disabled", port: undefined })
		await waitFor(() => relay.sessions === 0)
		await service.configure({ enabled: true })
		await waitFor(() => service.status().state === "connected")
		await expect(service.configure({ relay: "nope:abc" })).rejects.toThrow(/out of range/)
	})

	it("hangs up on a live session when its device is revoked", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel()
		await waitFor(() => service.status().state === "connected")
		const paired = service.addDevice("phone")
		const phone = await phoneConnect({
			port: service.status().port!,
			privateKey: paired.privateKey!,
		})
		phones.push(phone)
		expect(await httpThrough(phone, "127.0.0.1", ui.port)).toContain("ui ok")
		await waitFor(() => service.status().activeConnections === 1)

		const closed = new Promise<void>((resolve) => phone.on("close", () => resolve()))
		expect(service.removeDevice(paired.device.fingerprint)).toBe(true)
		// Revoking a lost device has to end the session it is already holding open,
		// not just refuse its next login.
		await closed
		await waitFor(() => service.status().activeConnections === 0)
		await expect(
			phoneConnect({ port: service.status().port!, privateKey: paired.privateKey! }),
		).rejects.toThrow()
	})

	it("counts a password attempt as a failed login but not the client's method probe", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel()
		await waitFor(() => service.status().state === "connected")
		const port = service.status().port!

		// Three password attempts is the per-connection limit; a client that only
		// ever offers passwords is trying a door that does not exist.
		await expect(
			new Promise((resolve, reject) => {
				const client = new SshClient()
				client.on("ready", () => resolve(undefined))
				client.on("error", reject)
				client.connect({
					host: "127.0.0.1",
					port,
					username: "simplex",
					password: "hunter2",
					readyTimeout: 5000,
					hostVerifier: () => true,
				})
			}),
		).rejects.toThrow()

		// The `none` probe every client opens with is part of the handshake, so a
		// paired device still connects afterwards rather than being rate-limited
		// out by its own greeting.
		const paired = service.addDevice("phone")
		const phone = await phoneConnect({ port, privateKey: paired.privateKey! })
		phones.push(phone)
		expect(await httpThrough(phone, "127.0.0.1", ui.port)).toContain("ui ok")
	})

	it("tells the device to forward to the address the UI is actually bound to", async () => {
		relay = new FakeRelay()
		await relay.start()
		// A UI on a specific non-loopback address: isTarget accepts only that
		// address, so a forward naming 127.0.0.1 is refused. The panel used to
		// advertise 127.0.0.1 regardless, which broke every such bind.
		const lan = await new Promise<{ server: HttpServer; port: number; host: string }>((resolve) => {
			const server = createHttpServer((_req, res) => res.end("lan ui ok"))
			server.listen(0, "0.0.0.0", () => {
				const port = (server.address() as { port: number }).port
				resolve({ server, port, host: "127.0.0.1" })
			})
		})
		try {
			const dir = mkdtempSync(join(tmpdir(), "simplex-bind-"))
			const service = new TunnelService({
				dataDir: dir,
				config: { enabled: true, relay: `127.0.0.1:${relay.port}` },
				// Stand-in for `--ui <specific-address>`: whatever host is given here
				// is the only one isTarget will accept.
				uiTarget: () => ({ host: lan.host, port: lan.port }),
				deliver: (socket) => {
					lan.server.emit("connection", socket)
					return true
				},
				maxBackoffMs: 200,
			})
			tunnel = service
			service.start()
			await waitFor(() => service.status().state === "connected")
			const paired = service.addDevice("phone")
			expect(paired.connection.localForward).toBe(`8686:${lan.host}:${lan.port}`)
			const phone = await phoneConnect({ port: service.status().port!, privateKey: paired.privateKey! })
			phones.push(phone)
			// What the panel shows is what the server accepts.
			const [, forwardHost, forwardPort] = paired.connection.localForward.split(":")
			expect(await httpThrough(phone, forwardHost, Number(forwardPort))).toContain("lan ui ok")
		} finally {
			lan.server.close()
		}
	})

	it("destroys the stream of a peer that fails auth and ignores the disconnect", async () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-hangup-"))
		const keys = new TunnelKeyStore(dir)
		const embedded = new EmbeddedSshServer({
			hostKey: keys.hostKey().privateKey,
			isAuthorized: () => false,
			target: () => ({ host: "127.0.0.1", port: ui.port }),
			deliver: () => true,
			maxAuthFailures: 1,
			authTimeoutMs: 60_000, // long, so only the failure limit can end this
		})
		const { clientSide, serverSide } = pipePair()
		embedded.inject(serverSide, { ip: "203.0.113.7", port: 40000 })
		const stranger = generateKeyPair()
		const client = new SshClient()
		client.on("error", () => {})
		// conn.end() closes our write side only; a peer whose stream ignores end()
		// used to keep the session, its protocol state and its slot in the
		// connection count alive for as long as it liked.
		client.connect({
			sock: stubborn(clientSide),
			username: "simplex",
			privateKey: stranger.private,
			hostVerifier: () => true,
		} as never)
		await waitFor(() => (serverSide as unknown as { destroyed: boolean }).destroyed, 8000)
		await waitFor(() => embedded.connections === 0, 8000)
		clientSide.destroy()
		embedded.close()
	})

	it("rejects a forged signature from an authorized key blob (algorithm confusion)", async () => {
		// The whole point of public-key auth is that knowing the public half buys
		// nothing. This is the regression test for the bypass where it did: ssh2's
		// key.verify() returns an Error (not false) on a critical failure, so a
		// loose `!key.verify(...)` accepted it. An attacker who knows a paired
		// device's *public* key offers that ed25519 blob tagged as an RSA signature
		// algorithm and a garbage signature — no private key. It must fail.
		const dir = mkdtempSync(join(tmpdir(), "simplex-forge-"))
		const keys = new TunnelKeyStore(dir)
		const victim = generateKeyPair()
		const victimBlob = utils.parseKey(victim.public).getPublicSSH()
		const victimFingerprint = fingerprintOf(victimBlob)
		const embedded = new EmbeddedSshServer({
			hostKey: keys.hostKey().privateKey,
			isAuthorized: (fp) => fp === victimFingerprint,
			target: () => ({ host: "127.0.0.1", port: ui.port }),
			deliver: () => true,
			// Let the client exhaust its one forged method and end its own
			// connection, rather than the server force-destroying the stream
			// mid-protocol — the latter races a late disconnect write into a dead
			// pipe (EPIPE) that would surface as an unhandled error in a later test.
			authTimeoutMs: 60_000,
		})
		const { clientSide, serverSide } = pipePair()
		embedded.inject(serverSide, { ip: "203.0.113.9", port: 40002 })

		// A "parsed key" whose type is ssh-rsa (so ssh2 signs with rsa-sha2-256,
		// hashAlgo sha256) but whose public blob is the victim's ed25519 key and
		// whose sign() returns garbage — exactly the exploit shape, no private key.
		const donor = utils.parseKey(utils.generateKeyPairSync("rsa", { bits: 2048 }).private)
		const forged = Object.create(Object.getPrototypeOf(donor))
		for (const sym of Object.getOwnPropertySymbols(donor)) forged[sym] = (donor as never)[sym]
		Object.assign(forged, {
			type: "ssh-rsa",
			getPublicSSH: () => victimBlob,
			getPublicPEM: () => donor.getPublicPEM(),
			sign: () => randomBytes(256),
			isPrivateKey: () => true,
		})

		const client = new SshClient()
		const authenticated = await new Promise<boolean>((resolve) => {
			client.on("ready", () => resolve(true))
			client.on("error", () => resolve(false))
			client.on("close", () => resolve(false))
			client.connect({
				sock: clientSide,
				username: "simplex",
				authHandler: () => ({ type: "publickey", username: "simplex", key: forged }),
				hostVerifier: () => true,
			} as never)
		})
		expect(authenticated).toBe(false)
		expect(embedded.connections).toBe(0)
		client.end()
		clientSide.destroy()
		embedded.close()
	})

	it("reaps a peer that never finishes the SSH identification line", async () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-ident-"))
		const keys = new TunnelKeyStore(dir)
		const embedded = new EmbeddedSshServer({
			hostKey: keys.hostKey().privateKey,
			isAuthorized: () => false,
			target: () => ({ host: "127.0.0.1", port: 1 }),
			deliver: () => true,
			authTimeoutMs: 300,
		})
		const { clientSide, serverSide } = pipePair()
		embedded.inject(serverSide, { ip: "203.0.113.8", port: 40001 })
		// A partial ident line: ssh2 raises no connection event for it, so this
		// stream reaches none of the per-connection timers.
		clientSide.write("SSH-2.0-evil")
		await waitFor(() => (serverSide as unknown as { destroyed: boolean }).destroyed, 5000)
		clientSide.destroy()
		embedded.close()
	})

	it("refuses a client that will only do an expensive key exchange", async () => {
		relay = new FakeRelay()
		await relay.start()
		const service = startTunnel()
		await waitFor(() => service.status().state === "connected")
		const paired = service.addDevice("phone")
		// group18 is an 8192-bit modulus: ~107ms of synchronous CPU per handshake
		// on the loop that fills orders, chosen by the client. It is no longer
		// offered, so a client that insists on it cannot connect at all.
		await expect(
			new Promise((resolve, reject) => {
				const client = new SshClient()
				client.on("ready", () => resolve(undefined))
				client.on("error", reject)
				client.connect({
					host: "127.0.0.1",
					port: service.status().port!,
					username: "simplex",
					privateKey: paired.privateKey!,
					readyTimeout: 5000,
					hostVerifier: () => true,
					algorithms: { kex: ["diffie-hellman-group18-sha512"] },
				} as never)
			}),
		).rejects.toThrow(/kex|algorithm|handshake/i)
		// A normal client is unaffected.
		const phone = await phoneConnect({ port: service.status().port!, privateKey: paired.privateKey! })
		phones.push(phone)
		expect(await httpThrough(phone, "127.0.0.1", ui.port)).toContain("ui ok")
	})

	it("does not pin a relay key until the handshake proves the relay holds it", async () => {
		relay = new FakeRelay()
		await relay.start()
		const dir = mkdtempSync(join(tmpdir(), "simplex-pin-order-"))
		const service = new TunnelService({
			dataDir: dir,
			config: { enabled: true, relay: `127.0.0.1:${relay.port}` },
			uiTarget: () => ({ host: "127.0.0.1", port: ui.port }),
			deliver: (socket) => {
				ui.server.emit("connection", socket)
				return true
			},
			maxBackoffMs: 200,
		})
		tunnel = service
		service.start()
		await waitFor(() => service.status().state === "connected")
		// Written only once the session is up. ssh2 calls hostVerifier while
		// handling KEXDH_REPLY, before the signature over the exchange hash is
		// checked, so anything pinned there is only a claim.
		expect(new TunnelKeyStore(dir).knownRelay(`127.0.0.1:${relay.port}`)).toEqual({
			relay: `127.0.0.1:${relay.port}`,
			fingerprint: relay.hostFingerprint,
		})
	})

	it("drops a configured host key pin when the relay changes", async () => {
		relay = new FakeRelay()
		await relay.start()
		const other = new FakeRelay()
		await other.start()
		try {
			// A pin set for one relay applied to every relay, so moving to another
			// one failed the check forever — with an error telling the operator to
			// delete a file that path never reads.
			const service = startTunnel({ relayHostKey: relay.hostFingerprint })
			await waitFor(() => service.status().state === "connected")
			await service.configure({ relay: `127.0.0.1:${other.port}` })
			await waitFor(() => service.status().state === "connected")
			expect(service.status().relayFingerprint).toBe(other.hostFingerprint)
		} finally {
			other.stop()
		}
	})

	it("refuses to let a paired device manage remote access", async () => {
		relay = new FakeRelay()
		await relay.start()
		// A UI server that answers like the real one: the tunnel guard lives in
		// UiServer.handle, so this stands in for it with the same rule.
		const seen: string[] = []
		const guarded = await new Promise<{ server: HttpServer; port: number }>((resolve) => {
			const server = createHttpServer((req, res) => {
				const path = (req.url ?? "/").split("?")[0]
				const method = req.method ?? "GET"
				seen.push(`${method} ${path}`)
				if (path.startsWith("/api/tunnel") && method !== "GET" && method !== "HEAD" && isTunnelled(req.socket)) {
					res.writeHead(403, { "Content-Type": "application/json" })
					return res.end(JSON.stringify({ error: "Remote access can only be changed from the machine running Simplex" }))
				}
				res.end(JSON.stringify({ ok: true, readOnly: isTunnelled(req.socket) }))
			})
			server.listen(0, "127.0.0.1", () => resolve({ server, port: (server.address() as { port: number }).port }))
		})
		try {
			const dir = mkdtempSync(join(tmpdir(), "simplex-guard-"))
			const service = new TunnelService({
				dataDir: dir,
				config: { enabled: true, relay: `127.0.0.1:${relay.port}` },
				uiTarget: () => ({ host: "127.0.0.1", port: guarded.port }),
				deliver: (socket) => {
					guarded.server.emit("connection", socket)
					return true
				},
				maxBackoffMs: 200,
			})
			tunnel = service
			service.start()
			await waitFor(() => service.status().state === "connected")
			const paired = service.addDevice("phone")
			const phone = await phoneConnect({ port: service.status().port!, privateKey: paired.privateKey! })
			phones.push(phone)

			// Pairing a second key over the tunnel would survive revoking this one.
			const pair = await httpThrough(phone, "127.0.0.1", guarded.port, {
				method: "POST",
				path: "/api/tunnel/devices",
			})
			expect(pair).toContain("403")
			expect(pair).toContain("can only be changed from the machine")

			// Reads still work, and say so, which is what lets the panel render
			// itself read-only instead of failing on the first click.
			const status = await httpThrough(phone, "127.0.0.1", guarded.port, { path: "/api/tunnel" })
			expect(status).toContain('"readOnly":true')

			// Everything else a device is meant to do is untouched.
			const other = await httpThrough(phone, "127.0.0.1", guarded.port, { method: "POST", path: "/api/send" })
			expect(other).toContain('"ok":true')
		} finally {
			guarded.server.close()
		}
	})

	it("surfaces a relay that refuses the forward", async () => {
		relay = new FakeRelay(false)
		await relay.start()
		const service = startTunnel()
		await waitFor(() => /refused the port forward/.test(service.status().lastError ?? ""))
		expect(service.status().state).not.toBe("connected")
	})
})
