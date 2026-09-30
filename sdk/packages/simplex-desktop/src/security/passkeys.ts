import { randomBytes } from "node:crypto"
import { createServer, type IncomingMessage, type RequestListener, type Server } from "node:http"
import {
	generateAuthenticationOptions,
	generateRegistrationOptions,
	verifyAuthenticationResponse,
	verifyRegistrationResponse,
	type AuthenticationResponseJSON,
	type RegistrationResponseJSON,
} from "@simplewebauthn/server"
import { passkeyPage } from "./passkey-page"

export interface PasskeyCredential {
	id: string
	publicKey: string
	counter: number
	transports?: string[]
	/** WebAuthn user handle; reusing it lets the authenticator replace this profile's passkey. */
	userId?: string
}
export interface PasskeyUnlock {
	available(): boolean
	register(userId?: string): Promise<PasskeyCredential>
	authenticate(credential: PasskeyCredential): Promise<PasskeyCredential>
	cancel(): void
}

export function isPasskeyCredential(value: unknown): value is PasskeyCredential {
	if (!value || typeof value !== "object") return false
	const credential = value as PasskeyCredential
	return (
		typeof credential.id === "string" &&
		/^[A-Za-z0-9_-]{1,2048}$/.test(credential.id) &&
		typeof credential.publicKey === "string" &&
		/^[A-Za-z0-9_-]{1,4096}$/.test(credential.publicKey) &&
		Number.isSafeInteger(credential.counter) &&
		credential.counter >= 0 &&
		(credential.userId === undefined ||
			(typeof credential.userId === "string" && /^[A-Za-z0-9_-]{1,86}$/.test(credential.userId))) &&
		(credential.transports === undefined ||
			(Array.isArray(credential.transports) &&
				credential.transports.length <= 8 &&
				credential.transports.every((transport) =>
					["ble", "cable", "hybrid", "internal", "nfc", "smart-card", "usb"].includes(transport),
				)))
	)
}

async function readBody(request: IncomingMessage): Promise<unknown> {
	let size = 0
	const chunks: Buffer[] = []
	for await (const chunk of request) {
		size += chunk.length
		if (size > 65_536) throw new Error("Passkey response too large")
		chunks.push(Buffer.from(chunk))
	}
	return JSON.parse(Buffer.concat(chunks).toString("utf8"))
}

function listen(server: Server, port: number, host: string): Promise<void> {
	return new Promise((resolve, reject) => {
		server.once("error", reject)
		server.listen(port, host, () => {
			server.off("error", reject)
			resolve()
		})
	})
}

/**
 * Browsers may resolve localhost to ::1 before 127.0.0.1, so serve both loopbacks on one port.
 * Another process holding the ::1 port could read the fragment token, so pick a new port instead.
 */
async function listenLoopback(handler: RequestListener, servers: Server[]): Promise<number> {
	for (let attempt = 0; attempt < 5; attempt++) {
		const ipv4 = createServer(handler)
		ipv4.requestTimeout = 10_000
		servers.push(ipv4)
		await listen(ipv4, 0, "127.0.0.1")
		const address = ipv4.address()
		if (!address || typeof address === "string") throw new Error("Could not start passkey login")
		const ipv6 = createServer(handler)
		ipv6.requestTimeout = 10_000
		try {
			await listen(ipv6, address.port, "::1")
			servers.push(ipv6)
			return address.port
		} catch (error) {
			const code = (error as NodeJS.ErrnoException).code
			if (code === "EADDRNOTAVAIL" || code === "EAFNOSUPPORT") return address.port
			if (code !== "EADDRINUSE") throw error
			servers.pop()?.close()
		}
	}
	throw new Error("Could not start passkey login")
}

/**
 * WebAuthn requires a web origin, so use the system browser at localhost rather than simplex:.
 * Each ceremony has a fresh port, capability token, challenge and exact expected origin.
 * The listener exposes only WebAuthn data, never the solver or encryption key.
 */
export class BrowserPasskeys implements PasskeyUnlock {
	private abort?: () => void
	private generation = 0
	constructor(
		private readonly options: {
			openBrowser(url: string): Promise<void>
			platform?: NodeJS.Platform
			timeoutMs?: number
		},
	) {}
	available(): boolean {
		return ["darwin", "win32"].includes(this.options.platform ?? process.platform)
	}
	cancel(): void {
		this.generation++
		this.abort?.()
	}
	async register(userId = randomBytes(16).toString("base64url")): Promise<PasskeyCredential> {
		const generation = this.generation
		const options = await generateRegistrationOptions({
			rpName: "Simplex Desktop",
			rpID: "localhost",
			userID: new Uint8Array(Buffer.from(userId, "base64url")),
			userName: `Simplex ${userId.slice(0, 8)}`,
			userDisplayName: "Simplex Desktop profile",
			attestationType: "none",
			authenticatorSelection: {
				authenticatorAttachment: "platform",
				residentKey: "required",
				userVerification: "required",
			},
			supportedAlgorithmIDs: [-7, -257],
			timeout: this.options.timeoutMs ?? 120_000,
		})
		if (generation !== this.generation) throw new Error("Passkey cancelled. Try again.")
		return this.ceremony("register", options, async (response, origin) => {
			const result = await verifyRegistrationResponse({
				response: response as RegistrationResponseJSON,
				expectedChallenge: options.challenge,
				expectedOrigin: origin,
				expectedRPID: "localhost",
				requireUserVerification: true,
				supportedAlgorithmIDs: [-7, -257],
			})
			if (!result.verified) throw new Error("Passkey registration failed")
			const credential = result.registrationInfo.credential
			return { ...credential, publicKey: Buffer.from(credential.publicKey).toString("base64url"), userId }
		})
	}
	async authenticate(credential: PasskeyCredential): Promise<PasskeyCredential> {
		const generation = this.generation
		const options = await generateAuthenticationOptions({
			rpID: "localhost",
			userVerification: "required",
			allowCredentials: [{ id: credential.id, transports: credential.transports }],
			timeout: this.options.timeoutMs ?? 120_000,
		})
		if (generation !== this.generation) throw new Error("Passkey cancelled. Try again.")
		return this.ceremony("authenticate", options, async (response, origin) => {
			const assertion = response as AuthenticationResponseJSON
			if (assertion.id !== credential.id) throw new Error("Unknown passkey")
			const result = await verifyAuthenticationResponse({
				response: assertion,
				credential: {
					...credential,
					publicKey: new Uint8Array(Buffer.from(credential.publicKey, "base64url")),
				},
				expectedChallenge: options.challenge,
				expectedOrigin: origin,
				expectedRPID: "localhost",
				requireUserVerification: true,
			})
			if (!result.verified) throw new Error("Passkey authentication failed")
			return { ...credential, counter: result.authenticationInfo.newCounter }
		})
	}
	private async ceremony(
		kind: "register" | "authenticate",
		options: { challenge: string },
		verify: (response: unknown, origin: string) => Promise<PasskeyCredential>,
	): Promise<PasskeyCredential> {
		if (!this.available()) throw new Error("Passkeys are available on macOS and Windows")
		if (this.abort) throw new Error("A passkey request is already in progress")
		const token = randomBytes(32).toString("hex")
		const nonce = randomBytes(16).toString("hex")
		let origin = ""
		let used = false
		let settled = false
		let timer: NodeJS.Timeout | undefined
		let resolve!: (credential: PasskeyCredential) => void
		let reject!: (error: Error) => void
		const result = new Promise<PasskeyCredential>((yes, no) => {
			resolve = yes
			reject = no
		})
		// Browser launch can fail before the caller begins awaiting the result.
		void result.catch(() => {})
		const finish = (error?: Error, credential?: PasskeyCredential) => {
			if (settled) return
			settled = true
			clearTimeout(timer)
			if (error) reject(error)
			else resolve(credential!)
		}
		const handler: RequestListener = async (request, response) => {
			response.setHeader("Cache-Control", "no-store")
			response.setHeader("Referrer-Policy", "no-referrer")
			response.setHeader("X-Content-Type-Options", "nosniff")
			response.setHeader(
				"Content-Security-Policy",
				`default-src 'none'; script-src 'nonce-${nonce}'; style-src 'nonce-${nonce}'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'`,
			)
			const send = (status: number, body: unknown) => {
				response.writeHead(status, { "Content-Type": "application/json" })
				response.end(JSON.stringify(body))
			}
			if (settled || !origin || request.headers.host !== new URL(origin).host)
				return send(403, { error: "Forbidden" })
			if (request.url === "/" && request.method === "GET") {
				response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" })
				response.end(passkeyPage(nonce))
				return
			}
			if (
				request.method !== "POST" ||
				request.headers.origin !== origin ||
				request.headers.authorization !== `Bearer ${token}` ||
				request.headers["content-type"] !== "application/json"
			)
				return send(403, { error: "Forbidden" })
			if (request.url === "/cancel") {
				response.once("finish", () =>
					finish(new Error("Passkey cancelled. Try again or use another sign-in method.")),
				)
				send(200, { ok: true })
				return
			}
			if (request.url === "/options" && !used) return send(200, { kind, options })
			if (request.url !== "/verify" || used) return send(400, { error: "Expired passkey request" })
			used = true // Consume the challenge even when verification fails.
			try {
				const credential = await verify(await readBody(request), origin)
				if (settled) return send(400, { error: "Expired passkey request" })
				response.once("finish", () => finish(undefined, credential))
				send(200, { ok: true })
			} catch {
				response.once("finish", () =>
					finish(new Error("Passkey verification failed. Try again or use another sign-in method.")),
				)
				send(400, { error: "Passkey verification failed" })
			}
		}
		const servers: Server[] = []
		this.abort = () => finish(new Error("Passkey cancelled. Try again or use another sign-in method."))
		try {
			origin = `http://localhost:${await listenLoopback(handler, servers)}`
			timer = setTimeout(
				() => finish(new Error("Passkey request expired. Try again.")),
				this.options.timeoutMs ?? 120_000,
			)
			if (settled) return await result
			await this.options.openBrowser(`${origin}/#${token}`)
			return await result
		} finally {
			clearTimeout(timer)
			for (const server of servers) {
				server.close()
				server.closeAllConnections()
			}
			this.abort = undefined
		}
	}
}
