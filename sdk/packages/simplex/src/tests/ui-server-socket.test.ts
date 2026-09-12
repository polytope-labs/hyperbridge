import { describe, it, expect, afterEach, vi } from "vitest"
import { existsSync, linkSync, mkdtempSync, readFileSync, statSync, symlinkSync, writeFileSync } from "node:fs"
import { request as httpRequest } from "node:http"
import { createServer as createNetServer, connect as netConnect, createConnection, type Socket } from "node:net"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { UiServer, type OperatorContext } from "@/services/server/UiServer"
import { ActivityRecorder } from "@/data/recorder"
import { MemoryDataStore } from "@/data/memory"
import { SignerType } from "@/services/wallet"
import type { FillerConfigFile } from "@/config/filler-toml"
import type { ActivityEvent } from "@/data/types"

// Covers the Unix-socket listen mode: an embedding application (the desktop app)
// attaches to the daemon over a `0600` socket file rather than a TCP port, so no
// other local user can reach `/api/send` and no web page can reach it at all.
//
// The socket path doubles as that application's discovery mechanism and its
// single-instance lock, which is why a leftover file must be recovered rather
// than fatal, and a live one must be refused rather than stolen.

const CSRF = { "X-Simplex-UI": "1", "Content-Type": "application/json" }

function tmpDir(): string {
	return mkdtempSync(join(tmpdir(), "simplex-uds-"))
}

/** One HTTP request over a Unix socket, with the Host header under our control. */
function socketRequest(
	socketPath: string,
	path: string,
	opts: { method?: string; headers?: Record<string, string>; body?: string } = {},
): Promise<{ status: number; body: string }> {
	return new Promise((resolve, reject) => {
		const req = httpRequest({ socketPath, path, method: opts.method ?? "GET", headers: opts.headers }, (res) => {
			let body = ""
			res.setEncoding("utf8")
			res.on("data", (chunk: string) => {
				body += chunk
			})
			res.on("end", () => resolve({ status: res.statusCode ?? 0, body }))
		})
		req.on("error", reject)
		req.end(opts.body)
	})
}

/** A raw GET over TCP, so the Host header is exactly what the test says it is. */
function tcpRequest(port: number, path: string, host: string): Promise<string> {
	return new Promise((resolve, reject) => {
		const socket = createConnection(port, "127.0.0.1", () => {
			socket.write(`GET ${path} HTTP/1.1\r\nHost: ${host}\r\nConnection: close\r\n\r\n`)
		})
		let data = ""
		socket.on("data", (chunk) => {
			data += chunk.toString()
		})
		socket.on("end", () => resolve(data))
		socket.on("error", reject)
	})
}

/** Opens the SSE stream over a socket and hands back the frames as they arrive. */
function openSse(socketPath: string): Promise<{
	status: number
	contentType?: string
	next: () => Promise<string>
	close: () => void
}> {
	return new Promise((resolve, reject) => {
		const req = httpRequest({ socketPath, path: "/api/events", method: "GET" }, (res) => {
			const pending: string[] = []
			let waiter: ((frame: string) => void) | undefined
			res.setEncoding("utf8")
			res.on("data", (chunk: string) => {
				if (waiter) {
					waiter(chunk)
					waiter = undefined
				} else pending.push(chunk)
			})
			resolve({
				status: res.statusCode ?? 0,
				contentType: res.headers["content-type"],
				next: () =>
					pending.length > 0
						? Promise.resolve(pending.shift() as string)
						: new Promise<string>((r) => {
								waiter = r
							}),
				close: () => req.destroy(),
			})
		})
		req.on("error", reject)
		req.end()
	})
}

/**
 * Hands the server a live socket it never listened for — the shape of every
 * tunnel connection, which arrives as an SSH channel dialled outbound to the
 * relay and injected with `accept()`. A real TCP pair rather than a stub duplex,
 * so the HTTP server gets a socket with every method it actually calls.
 */
async function injectConnection(server: UiServer): Promise<{ client: Socket; delivered: boolean }> {
	const pairer = createNetServer()
	await new Promise<void>((resolve) => pairer.listen(0, "127.0.0.1", () => resolve()))
	const { port } = pairer.address() as { port: number }
	const accepted = new Promise<Socket>((resolve) => pairer.once("connection", resolve))
	const client = netConnect(port, "127.0.0.1")
	await new Promise<void>((resolve, reject) => {
		client.once("connect", () => resolve())
		client.once("error", reject)
	})
	const serverSide = await accepted
	pairer.close()
	return { client, delivered: server.accept(serverSide) }
}

function readAll(socket: Socket): Promise<string> {
	return new Promise((resolve, reject) => {
		let data = ""
		socket.on("data", (chunk) => {
			data += chunk.toString()
		})
		socket.on("end", () => resolve(data))
		socket.on("close", () => resolve(data))
		socket.on("error", reject)
	})
}

function operatorContext(): OperatorContext & { configPath: string } {
	const data = new MemoryDataStore()
	const config: FillerConfigFile = {
		simplex: {
			signer: { type: SignerType.PrivateKey, key: "0xab" },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://example",
		},
		pairs: [],
		chains: [],
	}
	let paused = false
	return {
		strategies: [],
		filler: {
			pause() {
				paused = true
			},
			resume() {
				paused = false
			},
			isPaused: () => paused,
			getWatchOnly: () => ({}),
		},
		balances: { getSnapshot: () => ({ updatedAt: null, status: "loading", chains: [], issues: [] }) },
		haltControls: [],
		config,
		stop: vi.fn().mockResolvedValue(undefined),
		activity: new ActivityRecorder(data.activity),
		bids: data.bids,
		setPaused: vi.fn(),
		setLogLevel: vi.fn(),
		applyAllowlist: vi.fn(),
		applyRebalancing: vi.fn(),
		version: "0.0.0-test",
		startedAt: Date.now(),
		configPath: join(tmpDir(), "filler-config.toml"),
		chains: [],
		strategyTypes: [],
	}
}

describe("UiServer Unix socket listen mode", () => {
	const servers: UiServer[] = []

	function newServer(): { server: UiServer; operator: OperatorContext } {
		const operator = operatorContext()
		const server = new UiServer({ mode: "operator", operator })
		servers.push(server)
		return { server, operator }
	}

	afterEach(() => {
		while (servers.length > 0) servers.pop()?.stop()
	})

	it("serves the JSON API, mutating routes and SSE over a socket", async () => {
		const socketPath = join(tmpDir(), "ui.sock")
		const { server, operator } = newServer()
		expect(await server.start({ socketPath })).toBe(0)

		const health = await socketRequest(socketPath, "/health")
		expect(health.status).toBe(200)
		expect(JSON.parse(health.body)).toEqual({ status: "ok", mode: "operator" })

		const status = await socketRequest(socketPath, "/api/status")
		expect(status.status).toBe(200)
		expect(JSON.parse(status.body).mode).toBe("operator")

		// Mutating routes work, and the CSRF header rule is untouched by the transport.
		const unguarded = await socketRequest(socketPath, "/api/pause", { method: "POST" })
		expect(unguarded.status).toBe(403)
		const paused = await socketRequest(socketPath, "/api/pause", { method: "POST", headers: CSRF })
		expect(paused.status).toBe(200)
		expect(JSON.parse(paused.body)).toEqual({ paused: true })

		// SSE is the endpoint the desktop app has to bridge, so it gets its own check.
		const sse = await openSse(socketPath)
		expect(sse.status).toBe(200)
		expect(sse.contentType).toBe("text/event-stream")
		expect(await sse.next()).toBe(":ok\n\n")
		const event = { id: 1, ts: 42, type: "detected" } as unknown as ActivityEvent
		operator.activity.emit("event", event)
		expect(await sse.next()).toBe(`data: ${JSON.stringify(event)}\n\n`)
		sse.close()
	})

	it("skips the host-header check on a socket while TCP still enforces it", async () => {
		const socketPath = join(tmpDir(), "ui.sock")
		const { server } = newServer()
		await server.start({ socketPath })

		// An HTTP client over a socket puts whatever it likes in Host — node:http
		// sends "localhost", others send the URL's authority. There is no name to
		// rebind onto a socket and no browser that can open one, so the check is
		// skipped rather than relaxed, and a Host that TCP must refuse is served.
		for (const host of ["evil.example.com", "simplex.internal", "unix", "127.0.0.1.evil.example.com"]) {
			const res = await socketRequest(socketPath, "/api/status", { headers: { host } })
			expect({ host, status: res.status }).toEqual({ host, status: 200 })
		}

		// The DNS-rebinding defense is unchanged for a listening port.
		const { server: overTcp } = newServer()
		const port = await overTcp.start(0)
		expect(await tcpRequest(port, "/api/status", "evil.example.com")).toContain("403")
		expect(await tcpRequest(port, "/api/status", "127.0.0.1.evil.example.com")).toContain("403")
		expect(await tcpRequest(port, "/api/status", `127.0.0.1:${port}`)).toContain("200")
	})

	it("narrows the socket to the owning user and removes it on stop", async () => {
		// Windows named pipes carry no file mode; see docs/ai/Decisions.md.
		if (process.platform === "win32") return
		const socketPath = join(tmpDir(), "ui.sock")
		const { server } = newServer()
		await server.start({ socketPath })

		// Node binds it 0777 & ~umask — 0775 under the common `umask 002`, which
		// lets the operator's whole group drive /api/send. The chmod is the access
		// control this listen mode exists for, not decoration.
		expect((statSync(socketPath).mode & 0o777).toString(8)).toBe("600")

		server.stop()
		expect(existsSync(socketPath)).toBe(false)
	})

	it("creates the socket 0600 even under a permissive umask", async () => {
		// The mode IS the access control here, and it has to be right at creation, not
		// shortly after: libuv makes the file 0777 & ~umask, and Linux checks that mode
		// at connect(2) and never again — so a connection won before a later chmod is
		// served for the life of the daemon. Under `umask 0` the old bind-then-chmod
		// order left the socket briefly 0777 to every local user.
		if (process.platform === "win32") return
		const socketPath = join(tmpDir(), "ui.sock")
		const { server } = newServer()
		const previousUmask = process.umask(0o000)
		try {
			await server.start({ socketPath })
			expect((statSync(socketPath).mode & 0o777).toString(8)).toBe("600")
		} finally {
			process.umask(previousUmask)
		}
		// The process-wide umask is borrowed only for the bind, and handed back.
		expect(process.umask()).toBe(previousUmask)
	})

	it("refuses a non-socket at the socket path instead of deleting it", async () => {
		// connect(2) answers ECONNREFUSED for a regular file exactly as it does for an
		// orphaned socket, so a probe-only staleness test reads an operator's file as a
		// corpse and unlinks it. `--ui-socket ~/filler-config.toml` must not eat it.
		if (process.platform === "win32") return
		const socketPath = join(tmpDir(), "not-a-socket.toml")
		writeFileSync(socketPath, "[simplex.signer]\nkey = \"0xdeadbeef\"\n")
		const { server } = newServer()
		await expect(server.start({ socketPath })).rejects.toThrow(/exists and is a regular file; refusing to remove it/)
		expect(readFileSync(socketPath, "utf8")).toContain("0xdeadbeef")
	})

	it("refuses a dangling symlink at the socket path with an actionable error", async () => {
		// existsSync follows symlinks, so a broken one reads as absent: nothing gets
		// cleaned up and the bind then fails with a bare EADDRINUSE naming no cause.
		if (process.platform === "win32") return
		const dir = tmpDir()
		const socketPath = join(dir, "ui.sock")
		symlinkSync(join(dir, "nowhere"), socketPath)
		expect(existsSync(socketPath)).toBe(false)
		const { server } = newServer()
		await expect(server.start({ socketPath })).rejects.toThrow(/exists and is a symbolic link; refusing to remove it/)
	})

	it("does not relabel a live TCP listener's connections when a socket start fails", async () => {
		// `listenProvenance` decides whether the DNS-rebinding check runs. Setting it
		// before the bind meant a rejected socket start on an already-listening server
		// left every TCP connection tagged 'unix' — and so exempt from the Host check.
		const { server } = newServer()
		const port = await server.start(0)
		expect(await tcpRequest(port, "/api/status", "evil.example.com")).toContain("403")

		await expect(server.start({ socketPath: join(tmpDir(), "ui.sock") })).rejects.toThrow(/already listening/)

		// Still a TCP listener, so still defended.
		expect(await tcpRequest(port, "/api/status", "evil.example.com")).toContain("403")
		expect(await tcpRequest(port, "/api/status", `127.0.0.1:${port}`)).toContain("200")
	})

	it("recovers a socket file left behind by a killed run", async () => {
		const dir = tmpDir()
		const bound = join(dir, "bound.sock")
		const orphan = join(dir, "orphan.sock")

		// A genuine orphaned socket inode. Binding one and hard-linking a second
		// name to it, then closing the server, leaves the link behind: close()
		// unlinks only the name it bound. That is exactly the on-disk state a
		// SIGKILLed run leaves, and it is why an existence test will not do — a
		// live socket has a file too.
		const previous = createNetServer()
		await new Promise<void>((resolve) => previous.listen(bound, () => resolve()))
		linkSync(bound, orphan)
		await new Promise<void>((resolve) => previous.close(() => resolve()))
		expect(existsSync(orphan)).toBe(true)
		expect(statSync(orphan).isSocket()).toBe(true)

		const { server } = newServer()
		await expect(server.start({ socketPath: orphan })).resolves.toBe(0)
		expect((await socketRequest(orphan, "/health")).status).toBe(200)
	})

	it("refuses to take over a socket another instance is serving", async () => {
		const socketPath = join(tmpDir(), "ui.sock")
		const { server: first } = newServer()
		await first.start({ socketPath })

		// Stealing the path would silently take the running instance's clients.
		// Refusing is also the single-instance lock the desktop app attaches with.
		const { server: second } = newServer()
		await expect(second.start({ socketPath })).rejects.toThrow(/already serving/i)

		// The incumbent is untouched.
		expect((await socketRequest(socketPath, "/health")).status).toBe(200)
	})

	it("rejects a socket path longer than sun_path with an actionable error", async () => {
		// Otherwise this surfaces at bind time naming neither the limit nor the path.
		if (process.platform === "win32") return
		const socketPath = join(tmpDir(), `${"a".repeat(120)}.sock`)
		const { server } = newServer()
		await expect(server.start({ socketPath })).rejects.toThrow(/over this platform's \d+-byte limit/)
		expect(existsSync(socketPath)).toBe(false)
	})

	// The trap this pins: `accept()` returns false unless the server is listening,
	// which couples remote access to a listener it does not logically need. Tunnel
	// connections are dialled outbound to the relay and injected — nothing listens
	// for them — so a socket-only server must serve remote devices exactly as a TCP
	// one does. A Unix listener satisfies the guard today; a later change that drops
	// the listener entirely would break remote access, and this is what would catch it.
	it("serves tunnelled connections against a socket-only server", async () => {
		const socketPath = join(tmpDir(), "ui.sock")
		const { server } = newServer()
		await server.start({ socketPath })

		const { client, delivered } = await injectConnection(server)
		expect(delivered).toBe(true)
		client.write("GET /api/status HTTP/1.1\r\nHost: 127.0.0.1:8686\r\nConnection: close\r\n\r\n")
		const response = await readAll(client)
		expect(response).toContain("200 OK")
		expect(response).toContain('"mode":"operator"')
	})

	it("turns a tunnelled connection away when nothing is listening", async () => {
		// Not a silent failure, but the log it produces used to say "No UI to serve
		// behind the tunnel" — misleading, since the UI exists and is merely unbound.
		const { server } = newServer()
		const { client, delivered } = await injectConnection(server)
		expect(delivered).toBe(false)
		client.destroy()

		const socketPath = join(tmpDir(), "ui.sock")
		await server.start({ socketPath })
		server.stop()
		const afterStop = await injectConnection(server)
		expect(afterStop.delivered).toBe(false)
		afterStop.client.destroy()
	})
})
