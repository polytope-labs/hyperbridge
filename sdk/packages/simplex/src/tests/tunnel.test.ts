import { describe, it, expect, afterEach, beforeAll } from "vitest"
import { createServer as createHttpServer, type Server as HttpServer } from "node:http"
import { createServer as createTcpServer, type Server as TcpServer } from "node:net"
import { mkdtempSync, readFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import ssh2, {
	type Client as SshClientType,
	type ClientChannel,
	type Connection,
	type Server as SshServerType,
} from "ssh2"
import { TunnelService, parseRelayAddress, DEFAULT_TUNNEL_RELAY } from "@/services/tunnel/TunnelService"
import { TunnelKeyStore, fingerprintOf, fingerprintOfKeyText, normalizePublicKey } from "@/services/tunnel/keys"

const { Client: SshClient, Server: SshServer, utils } = ssh2

/**
 * A stand-in for simplex-tunnel, the Rust relay: accepts any public key,
 * opens a loopback port on `tcpip-forward`, and pipes every connection on it
 * back to the operator as a `forwarded-tcpip` channel with the requested bind
 * address echoed verbatim (which is what ssh2 matches channels on).
 */
class FakeRelay {
	readonly hostKey = utils.generateKeyPairSync("ed25519")
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

/** Sends one raw HTTP request through a direct-tcpip channel and returns the response text. */
function httpThrough(client: SshClientType, host: string, port: number): Promise<string> {
	return new Promise((resolve, reject) => {
		client.forwardOut("127.0.0.1", 0, host, port, (err: Error | undefined, stream: ClientChannel) => {
			if (err) return reject(err)
			let body = ""
			stream.on("data", (chunk: Buffer) => {
				body += chunk.toString()
			})
			stream.on("close", () => resolve(body))
			stream.on("error", reject)
			stream.end(`GET /health HTTP/1.1\r\nHost: localhost:8686\r\nConnection: close\r\n\r\n`)
		})
	})
}

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

		expect(store.knownRelay()).toBeUndefined()
		store.rememberRelay("relay:443", "SHA256:abc")
		expect(store.knownRelay()).toEqual({ relay: "relay:443", fingerprint: "SHA256:abc" })
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

describe("TunnelService", () => {
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
		expect(new TunnelKeyStore(dir).knownRelay()).toEqual({
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

	it("surfaces a relay that refuses the forward", async () => {
		relay = new FakeRelay(false)
		await relay.start()
		const service = startTunnel()
		await waitFor(() => /refused the port forward/.test(service.status().lastError ?? ""))
		expect(service.status().state).not.toBe("connected")
	})
})
