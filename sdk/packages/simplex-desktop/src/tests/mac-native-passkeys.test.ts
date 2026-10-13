import { describe, expect, it, vi } from "vitest"
import { MacNativePasskeys, type MacPasskeyBridge } from "../security/mac-native-passkeys"
import { isPasskeyCredential, isPasskeyRpId } from "../security/passkeys"
import { authenticator } from "./helpers/passkey-authenticator"

const rpId = "passkeys.example.com"
function fixture(overrides: Record<string, unknown> = {}) {
	const device = authenticator()
	let userId: string | undefined
	let counter = 0
	const requests: Record<string, string>[] = []
	const bridge: MacPasskeyBridge = {
		capabilities: vi.fn(() => ({ available: true })),
		cancel: vi.fn(),
		begin: vi.fn(async (json) => {
			const request = JSON.parse(json)
			requests.push(request)
			if (request.kind === "register") userId = request.userId
			return JSON.stringify(
				device.response(request.kind, request.challenge, `https://${rpId}`, {
					rp: rpId,
					userHandle: request.kind === "authenticate" ? userId : undefined,
					counter: request.kind === "register" ? 0 : ++counter,
					...overrides,
				}),
			)
		}),
	}
	const passkeys = new MacNativePasskeys({ rpId, bridge, windowHandle: () => Buffer.alloc(8) })
	return { passkeys, bridge, requests }
}

describe("native Mac passkey verification", () => {
	it("reports the native readiness reason and clears completed status for a new access flow", async () => {
		const f = fixture()
		vi.mocked(f.bridge.capabilities).mockReturnValueOnce({ available: false, reason: "macOS 13 or later required" })
		expect(f.passkeys.capability()).toEqual({ available: false, reason: "macOS 13 or later required" })
		await f.passkeys.register()
		expect(f.passkeys.status()).toMatchObject({ state: "succeeded", operationId: expect.any(String) })
		f.passkeys.resetStatus()
		expect(f.passkeys.status()).toEqual({
			mechanism: "native-macos",
			state: "idle",
			operationId: undefined,
			errorCode: undefined,
		})
	})
	it("labels entries with a stable profile and enrollment time while keeping replacement handles fresh", async () => {
		const f = fixture()
		const context = { profileId: "stable-profile-id" }
		const first = await f.passkeys.register(undefined, context)
		const second = await f.passkeys.register(undefined, context)
		for (const request of f.requests)
			expect(request.userName).toMatch(/^Simplex profile stable-p \(\d{4}-\d{2}-\d{2}T/)
		expect(first.userId).not.toBe(second.userId)
	})
	it("accepts legitimate zero counters without disabling signature verification", async () => {
		const f = fixture({ counter: 0 })
		const credential = await f.passkeys.register()
		await expect(f.passkeys.authenticate(credential)).resolves.toMatchObject({ counter: 0 })
	})
	it("verifies real registration and assertion signatures with a pinned domain and fresh challenges", async () => {
		const f = fixture()
		const credential = await f.passkeys.register()
		expect(isPasskeyCredential(credential)).toBe(true)
		expect(credential).toMatchObject({ mechanism: "native-macos", rpId })
		const verified = await f.passkeys.authenticate(credential)
		expect(verified.counter).toBe(1)
		expect(f.requests[1].credentialId).toBe(credential.id)
		expect(f.requests[0].challenge).not.toBe(f.requests[1].challenge)
		expect(f.passkeys.status().state).toBe("succeeded")
		const replacement = await f.passkeys.register()
		expect(replacement.userId).not.toBe(credential.userId)
	})
	it.each([{ rp: "attacker.example.com" }, { uv: false }])(
		"rejects an untrusted signed registration: %j",
		async (overrides) => {
			await expect(fixture(overrides).passkeys.register()).rejects.toThrow()
		},
	)
	it("rejects wrong origins, stale challenges, changed signatures and user handles", async () => {
		for (const field of ["origin", "challenge", "signature", "userHandle"]) {
			const f = fixture()
			const credential = await f.passkeys.register()
			const original = f.bridge.begin
			f.bridge.begin = vi.fn(async (...args: Parameters<MacPasskeyBridge["begin"]>) => {
				const response = JSON.parse(await original(...args))
				if (field === "origin" || field === "challenge") {
					const client = JSON.parse(Buffer.from(response.response.clientDataJSON, "base64url").toString())
					client[field] = field === "origin" ? "https://attacker.example.com" : "expired-challenge"
					response.response.clientDataJSON = Buffer.from(JSON.stringify(client)).toString("base64url")
				} else response.response[field] = "invalid"
				return JSON.stringify(response)
			})
			await expect(f.passkeys.authenticate(credential)).rejects.toThrow()
		}
	})
	it("never presents legacy localhost credentials and reports missing configuration without a prompt", async () => {
		const f = fixture()
		await expect(f.passkeys.authenticate({ id: "legacy", publicKey: "legacy", counter: 0 })).rejects.toThrow(
			/migration/,
		)
		expect(f.bridge.begin).not.toHaveBeenCalled()
		const missing = new MacNativePasskeys({ bridge: f.bridge, windowHandle: () => Buffer.alloc(8) })
		expect(missing.available()).toBe(false)
		await expect(missing.register()).rejects.toThrow(/configured/)
		expect(f.bridge.begin).not.toHaveBeenCalled()
	})
	it("cancels immediately, ignores a late native response and permits retry", async () => {
		const f = fixture()
		const original = f.bridge.begin
		let complete!: () => void
		f.bridge.begin = vi.fn(
			(...args: Parameters<MacPasskeyBridge["begin"]>) =>
				new Promise<string>((resolve) => {
					complete = async () => resolve(await original(...args))
				}),
		)
		const pending = f.passkeys.register()
		const rejected = expect(pending).rejects.toThrow(/cancelled/)
		await vi.waitFor(() => expect(f.bridge.begin).toHaveBeenCalledOnce())
		await expect(f.passkeys.register()).rejects.toThrow(/in progress/)
		f.passkeys.cancel()
		await rejected
		expect(f.bridge.cancel).toHaveBeenCalledOnce()
		complete()
		f.bridge.begin = original
		await expect(f.passkeys.register()).resolves.toMatchObject({ rpId })
	})
	it("expires a stalled native request", async () => {
		const f = fixture()
		f.bridge.begin = vi.fn(() => new Promise<string>(() => {}))
		const passkeys = new MacNativePasskeys({
			rpId,
			bridge: f.bridge,
			windowHandle: () => Buffer.alloc(8),
			timeoutMs: 20,
		})
		await expect(passkeys.register()).rejects.toThrow(/expired/)
		expect(f.bridge.cancel).toHaveBeenCalledOnce()
	})
	it.each([
		"localhost",
		"https://example.com",
		"127.0.0.1",
		"EXAMPLE.com",
		"-invalid.example",
		"example.com/",
		"example..com",
	])("rejects unsafe RP configuration %s", (rp) => {
		expect(isPasskeyRpId(rp)).toBe(false)
	})
})
