import { request as httpRequest } from "node:http"
import { createHash, generateKeyPairSync, randomBytes, sign } from "node:crypto"
import { isoCBOR } from "@simplewebauthn/server/helpers"
import { describe, expect, it, vi } from "vitest"
import { BrowserPasskeys, type PasskeyCredential } from "../security/passkeys"
import { passkeyPage } from "../security/passkey-page"

const hash = (data: string | Buffer) => createHash("sha256").update(data).digest()
function authenticator() {
	const { privateKey, publicKey } = generateKeyPairSync("ec", { namedCurve: "prime256v1" })
	const jwk = publicKey.export({ format: "jwk" })
	const id = randomBytes(32)
	const cose = Buffer.from(
		isoCBOR.encode(
			new Map<number, number | Uint8Array>([
				[1, 2],
				[3, -7],
				[-1, 1],
				[-2, Buffer.from(jwk.x!, "base64url")],
				[-3, Buffer.from(jwk.y!, "base64url")],
			]),
		),
	)
	const credential: PasskeyCredential = {
		id: id.toString("base64url"),
		publicKey: cose.toString("base64url"),
		counter: 0,
		transports: ["internal"],
	}
	function response(
		kind: string,
		challenge: string,
		origin: string,
		{ rp = "localhost", uv = true, counter = 1 } = {},
	) {
		const clientData = Buffer.from(
			JSON.stringify({ type: kind === "register" ? "webauthn.create" : "webauthn.get", challenge, origin }),
		)
		const header = Buffer.alloc(5)
		header[0] = 1 | (uv ? 4 : 0) | (kind === "register" ? 64 : 0)
		header.writeUInt32BE(kind === "register" ? 0 : counter, 1)
		const base = Buffer.concat([hash(rp), header])
		const length = Buffer.alloc(2)
		length.writeUInt16BE(id.length)
		const authData = kind === "register" ? Buffer.concat([base, Buffer.alloc(16), length, id, cose]) : base
		const common = {
			id: credential.id,
			rawId: credential.id,
			type: "public-key",
			authenticatorAttachment: "platform",
			clientExtensionResults: {},
		}
		return {
			...common,
			response:
				kind === "register"
					? {
							clientDataJSON: clientData.toString("base64url"),
							transports: ["internal"],
							attestationObject: Buffer.from(
								isoCBOR.encode(
									new Map<string, unknown>([
										["fmt", "none"],
										["attStmt", new Map()],
										["authData", authData],
									]) as Parameters<typeof isoCBOR.encode>[0],
								),
							).toString("base64url"),
						}
					: {
							clientDataJSON: clientData.toString("base64url"),
							authenticatorData: authData.toString("base64url"),
							signature: sign("sha256", Buffer.concat([authData, hash(clientData)]), privateKey).toString(
								"base64url",
							),
						},
		}
	}
	return { credential, response }
}

function browser(url: string) {
	const parsed = new URL(url)
	const origin = parsed.origin
	const headers = {
		Origin: origin,
		Authorization: `Bearer ${parsed.hash.slice(1)}`,
		"Content-Type": "application/json",
	}
	const post = (path: string, body: unknown = {}, overrides: Record<string, string> = {}) =>
		fetch(`${origin}${path}`, { method: "POST", headers: { ...headers, ...overrides }, body: JSON.stringify(body) })
	return { origin, headers, post }
}

describe("browser passkey ceremonies", () => {
	it("verifies real registration and signed authentication responses, with a fresh origin and challenge each time", async () => {
		const device = authenticator()
		const challenges: string[] = []
		const origins: string[] = []
		const passkeys = new BrowserPasskeys({
			platform: "win32",
			openBrowser: async (url) => {
				const b = browser(url)
				origins.push(b.origin)
				const { kind, options } = await (await b.post("/options")).json()
				challenges.push(options.challenge)
				expect(options.userVerification ?? options.authenticatorSelection.userVerification).toBe("required")
				if (kind === "register") {
					expect(options.authenticatorSelection.authenticatorAttachment).toBe("platform")
					expect(options.authenticatorSelection.residentKey).toBe("required")
				}
				expect((await b.post("/verify", device.response(kind, options.challenge, b.origin))).status).toBe(200)
			},
		})
		const credential = await passkeys.register()
		expect(credential).toEqual(device.credential)
		const authenticated = await passkeys.authenticate(credential)
		expect(authenticated.counter).toBe(1)
		expect(new Set(challenges).size).toBe(2)
		expect(new Set(origins).size).toBe(2)
		await expect(fetch(origins[1])).rejects.toThrow()
	})

	it.each(["challenge", "origin", "rp", "uv", "signature", "id", "counter"])(
		"rejects invalid authentication: %s",
		async (invalid) => {
			const device = authenticator()
			const passkeys = new BrowserPasskeys({
				platform: "darwin",
				openBrowser: async (url) => {
					const b = browser(url)
					const { kind, options } = await (await b.post("/options")).json()
					const response = device.response(
						kind,
						invalid === "challenge" ? "wrong-challenge" : options.challenge,
						invalid === "origin" ? "https://evil.example" : b.origin,
						{
							rp: invalid === "rp" ? "evil.example" : "localhost",
							uv: invalid !== "uv",
							counter: invalid === "counter" ? 0 : 1,
						},
					)
					if (invalid === "signature") response.response.signature = randomBytes(64).toString("base64url")
					if (invalid === "id") response.id = "another-credential"
					expect((await b.post("/verify", response)).status).toBe(400)
				},
			})
			await expect(
				passkeys.authenticate({ ...device.credential, counter: invalid === "counter" ? 1 : 0 }),
			).rejects.toThrow(/verification failed/)
		},
	)

	it.each(["challenge", "origin", "rp", "uv"])("rejects invalid registration: %s", async (invalid) => {
		const device = authenticator()
		const passkeys = new BrowserPasskeys({
			platform: "darwin",
			openBrowser: async (url) => {
				const b = browser(url)
				const { kind, options } = await (await b.post("/options")).json()
				expect(
					(
						await b.post(
							"/verify",
							device.response(
								kind,
								invalid === "challenge" ? "wrong" : options.challenge,
								invalid === "origin" ? "https://evil.example" : b.origin,
								{ rp: invalid === "rp" ? "evil.example" : "localhost", uv: invalid !== "uv" },
							),
						)
					).status,
				).toBe(400)
			},
		})
		await expect(passkeys.register()).rejects.toThrow(/verification failed/)
	})

	it("enforces the host, exact origin and capability token without consuming a valid session", async () => {
		const device = authenticator()
		const passkeys = new BrowserPasskeys({
			platform: "darwin",
			openBrowser: async (url) => {
				const b = browser(url)
				const page = await fetch(b.origin)
				expect(page.headers.get("cache-control")).toBe("no-store")
				expect(page.headers.get("content-security-policy")).toContain("frame-ancestors 'none'")
				expect(await page.text()).not.toContain(b.headers.Authorization)
				expect((await b.post("/options", {}, { Origin: "https://evil.example" })).status).toBe(403)
				expect((await b.post("/verify", {}, { Authorization: "Bearer wrong" })).status).toBe(403)
				const badHost = await new Promise<number>((resolve, reject) => {
					const request = httpRequest(
						`${b.origin}/cancel`,
						{ method: "POST", headers: { ...b.headers, Host: "evil.example" } },
						(response) => {
							response.resume()
							response.on("end", () => resolve(response.statusCode!))
						},
					)
					request.on("error", reject)
					request.end("{}")
				})
				expect(badHost).toBe(403)
				expect((await fetch(`${b.origin}/options`, { headers: b.headers })).status).toBe(403)
				const { kind, options } = await (await b.post("/options")).json()
				expect((await b.post("/verify", device.response(kind, options.challenge, b.origin))).status).toBe(200)
			},
		})
		await expect(passkeys.register()).resolves.toEqual(device.credential)
	})

	it("rejects replayed assertions in a new session", async () => {
		const device = authenticator()
		let assertion: ReturnType<typeof device.response> | undefined
		const passkeys = new BrowserPasskeys({
			platform: "darwin",
			openBrowser: async (url) => {
				const b = browser(url)
				const { kind, options } = await (await b.post("/options")).json()
				const replay = Boolean(assertion)
				assertion ??= device.response(kind, options.challenge, b.origin)
				expect((await b.post("/verify", assertion)).status).toBe(replay ? 400 : 200)
			},
		})
		await passkeys.authenticate(device.credential)
		await expect(passkeys.authenticate(device.credential)).rejects.toThrow(/verification failed/)
	})

	it("closes cancelled and expired listeners and can retry after browser launch failure", async () => {
		let url = ""
		const passkeys = new BrowserPasskeys({
			platform: "win32",
			timeoutMs: 40,
			openBrowser: async (next) => {
				url = next
			},
		})
		await expect(passkeys.register()).rejects.toThrow(/expired/)
		await expect(fetch(new URL(url).origin)).rejects.toThrow()
		const openBrowser = vi.fn(async (next: string): Promise<void> => {
			url = next
			throw new Error("browser missing")
		})
		const retry = new BrowserPasskeys({ platform: "win32", openBrowser })
		await expect(retry.register()).rejects.toThrow("browser missing")
		openBrowser.mockImplementation(async () => retry.cancel())
		await expect(retry.register()).rejects.toThrow(/cancelled/)
		await expect(fetch(new URL(url).origin)).rejects.toThrow()
	})

	it("bounds responses, handles browser cancellation and rejects concurrent ceremonies", async () => {
		const passkeys = new BrowserPasskeys({
			platform: "darwin",
			openBrowser: async (url) => {
				const b = browser(url)
				await expect(passkeys.register()).rejects.toThrow(/already in progress/)
				expect((await b.post("/verify", { padding: "x".repeat(65_537) })).status).toBe(400)
			},
		})
		await expect(passkeys.register()).rejects.toThrow(/verification failed/)
		const cancelled = new BrowserPasskeys({
			platform: "darwin",
			openBrowser: async (url) => {
				expect((await browser(url).post("/cancel")).status).toBe(200)
			},
		})
		await expect(cancelled.register()).rejects.toThrow(/cancelled/)
	})

	it("cancels before opening the browser if options are still being generated", async () => {
		const openBrowser = vi.fn()
		const passkeys = new BrowserPasskeys({ platform: "darwin", openBrowser })
		const registration = passkeys.register()
		passkeys.cancel()
		await expect(registration).rejects.toThrow(/cancelled/)
		expect(openBrowser).not.toHaveBeenCalled()
	})

	it("does not expose passkeys on Linux and serves valid browser JavaScript", async () => {
		const openBrowser = vi.fn()
		const passkeys = new BrowserPasskeys({ platform: "linux", openBrowser })
		expect(passkeys.available()).toBe(false)
		await expect(passkeys.register()).rejects.toThrow(/macOS and Windows/)
		expect(openBrowser).not.toHaveBeenCalled()
		const script = passkeyPage("nonce").match(/<script[^>]*>([\s\S]*)<\/script>/)![1]
		expect(() => new Function(script)).not.toThrow()
	})
})
