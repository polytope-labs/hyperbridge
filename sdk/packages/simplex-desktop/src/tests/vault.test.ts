import { mkdtempSync, readFileSync, readdirSync, rmSync, unlinkSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it, vi } from "vitest"
import { decryptConfig, encryptedConfigStore, isEncryptedConfig } from "@hyperbridge/simplex/config-storage"
import { DesktopVault, type BiometricUnlock } from "../security/vault"
import type { DeviceKeyStore } from "../security/device-key-store"

const password = "test-only password 1330"
const directories: string[] = []
afterEach(() => {
	vi.restoreAllMocks()
	for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
})
function directory() {
	const path = mkdtempSync(join(tmpdir(), "simplex-vault-"))
	directories.push(path)
	return path
}
function fixture(dataDir = directory(), extra: Partial<ConstructorParameters<typeof DesktopVault>[0]> = {}) {
	const start = vi.fn(async (_key: Buffer) => {})
	const prepare = vi.fn(async (_restartAllowed: boolean) => {})
	const biometrics: BiometricUnlock = {
		available: vi.fn(async () => false),
		confirm: vi.fn(async () => {}),
		protect: vi.fn(async () => "aabb"),
		unprotect: vi.fn(async () => Buffer.alloc(32)),
	}
	const deviceKeyStore: DeviceKeyStore = {
		available: vi.fn(async () => true),
		protect: vi.fn(async (key: Buffer) => Buffer.from(key.map((byte) => byte ^ 0xa5)).toString("hex")),
		unprotect: vi.fn(async (value: string) => Buffer.from(Buffer.from(value, "hex").map((byte) => byte ^ 0xa5))),
	}
	const vault = new DesktopVault({
		dataDir,
		start,
		prepare,
		biometrics,
		deviceKeyStore,
		needsRestart: async () => false,
		...extra,
	})
	return {
		vault,
		unlock: async (request: Parameters<DesktopVault["unlock"]>[0]) => {
			await vault.unlock(request)
			if ((await vault.state()).mode === "save-recovery") await vault.confirmRecovery(true)
		},
		dataDir,
		start,
		prepare,
		biometrics,
		deviceKeyStore,
		path: join(dataDir, "filler-config.toml"),
		metadata: join(dataDir, "desktop-vault.json"),
	}
}
const create = { method: "create", password, confirmation: password }
const replacement = { password: "replacement-password-1330", confirmation: "replacement-password-1330" }
async function enroll(f: ReturnType<typeof fixture>, useBiometrics = false) {
	await f.vault.unlock({ ...create, useBiometrics })
	const code = f.vault.recoveryCode()
	await f.vault.confirmRecovery(true)
	return code
}

describe("desktop config vault", () => {
	it("stores a separate OS wrapper and resumes the solver key while the UI stays locked", async () => {
		const f = fixture()
		const code = await enroll(f)
		const record = JSON.parse(readFileSync(f.metadata, "utf8"))
		expect(record.deviceKey).toMatch(/^[a-f0-9]{64}$/)
		expect(record.deviceKey).not.toBe(record.biometricKey)
		expect(record.deviceKey).not.toBe(code.replace(/-/g, "").toLowerCase())
		const relaunch = fixture(f.dataDir)
		const key = await relaunch.vault.backgroundKey()
		expect(key).toEqual(f.start.mock.calls[0][0])
		expect(relaunch.vault.isUnlocked()).toBe(false)
		expect(relaunch.start).not.toHaveBeenCalled()
		expect((await relaunch.vault.state()).backgroundResumeAvailable).toBe(true)
		key?.fill(0)
	})

	it("leaves profiles locked when secure OS storage is absent and enrolls it on the next password login", async () => {
		const f = fixture()
		f.deviceKeyStore.available = vi.fn(async () => false)
		await enroll(f)
		expect(JSON.parse(readFileSync(f.metadata, "utf8")).deviceKey).toBeUndefined()
		const relaunch = fixture(f.dataDir)
		relaunch.deviceKeyStore.available = vi.fn(async () => false)
		expect(await relaunch.vault.backgroundKey()).toBeUndefined()
		expect((await relaunch.vault.state()).secureDeviceStorageAvailable).toBe(false)
		relaunch.deviceKeyStore.available = vi.fn(async () => true)
		await relaunch.unlock({ method: "password", password })
		expect(JSON.parse(readFileSync(f.metadata, "utf8")).deviceKey).toMatch(/^[a-f0-9]{64}$/)
	})

	it("does not start or write anything until the recovery code is acknowledged", async () => {
		const f = fixture()
		await f.vault.unlock(create)
		expect((await f.vault.state()).mode).toBe("save-recovery")
		expect(f.vault.isUnlocked()).toBe(false)
		expect(f.start).not.toHaveBeenCalled()
		expect(readdirSync(f.dataDir)).toEqual([])
		const code = f.vault.recoveryCode()
		expect(code).toMatch(/^(?:[A-F0-9]{8}-){7}[A-F0-9]{8}$/)
		expect(f.vault.recoveryCode()).toBe(code)
		await expect(f.vault.confirmRecovery(false)).rejects.toThrow(/Save your recovery code/)
		await f.vault.confirmRecovery(true)
		expect((await f.vault.state()).recoveryEnabled).toBe(true)
		expect(f.vault.isUnlocked()).toBe(true)
		expect(readFileSync(f.metadata, "utf8").toLowerCase()).not.toContain(code.replace(/-/g, "").toLowerCase())
		expect(() => f.vault.recoveryCode()).toThrow()
	})

	it("resets a forgotten password with a code, rotates that code, and keeps the same encrypted config", async () => {
		const f = fixture()
		writeFileSync(f.path, "wallet credentials")
		const code = await enroll(f)
		const ciphertext = readFileSync(f.path, "utf8")
		const metadata = readFileSync(f.metadata, "utf8")
		const reset = fixture(f.dataDir)
		await expect(reset.vault.resetPassword(replacement)).rejects.toThrow(/Verify/)
		await reset.vault.recover({ method: "code", recoveryCode: code.toLowerCase().replace(/-/g, " ") })
		expect((await reset.vault.state()).mode).toBe("reset-password")
		expect(reset.vault.isUnlocked()).toBe(false)
		expect(reset.prepare).not.toHaveBeenCalled()
		await expect(reset.vault.resetPassword({ ...replacement, confirmation: "wrong" })).rejects.toThrow(/match/)
		await reset.vault.resetPassword(replacement)
		const newCode = reset.vault.recoveryCode()
		expect(newCode).not.toBe(code)
		expect(readFileSync(f.metadata, "utf8")).toBe(metadata)
		await reset.vault.confirmRecovery(true)
		expect(readFileSync(f.path, "utf8")).toBe(ciphertext)
		expect(reset.start.mock.calls[0][0]).toEqual(f.start.mock.calls[0][0])
		await expect(fixture(f.dataDir).vault.unlock({ method: "password", password })).rejects.toThrow(/Incorrect/)
		await expect(fixture(f.dataDir).vault.recover({ method: "code", recoveryCode: code })).rejects.toThrow(
			/Invalid/,
		)
		await fixture(f.dataDir).unlock({ method: "password", password: replacement.password })
		const recovered = fixture(f.dataDir)
		await recovered.vault.recover({ method: "code", recoveryCode: newCode })
		expect((await recovered.vault.state()).mode).toBe("reset-password")
	})

	it("rejects wrong codes and keeps failed authentication away from the solver", async () => {
		const f = fixture()
		await enroll(f)
		const before = readFileSync(f.metadata, "utf8")
		const next = fixture(f.dataDir)
		await expect(next.vault.recover({ method: "code", recoveryCode: "00".repeat(32) })).rejects.toThrow(/Invalid/)
		expect(next.prepare).not.toHaveBeenCalled()
		expect(next.start).not.toHaveBeenCalled()
		expect(next.vault.isUnlocked()).toBe(false)
		expect(readFileSync(f.metadata, "utf8")).toBe(before)
	})

	it("cancels and expires recovery without replacing the password or code", async () => {
		const f = fixture()
		const code = await enroll(f)
		const before = readFileSync(f.metadata, "utf8")
		const next = fixture(f.dataDir)
		await next.vault.recover({ method: "code", recoveryCode: code })
		await next.vault.resetPassword(replacement)
		await next.vault.cancelRecovery()
		expect((await next.vault.state()).mode).toBe("unlock")
		expect(readFileSync(f.metadata, "utf8")).toBe(before)
		const expired = fixture(f.dataDir)
		await expired.vault.recover({ method: "code", recoveryCode: code })
		vi.spyOn(Date, "now").mockReturnValue(Date.now() + 600_001)
		await expect(expired.vault.resetPassword(replacement)).rejects.toThrow(/Verify/)
		expect(readFileSync(f.metadata, "utf8")).toBe(before)
		await fixture(f.dataDir).unlock({ method: "password", password })
	})

	it("rejects security files without a recovery wrapper or with malformed JSON", async () => {
		const f = fixture()
		await enroll(f)
		const record = JSON.parse(readFileSync(f.metadata, "utf8"))
		delete record.recoveryKey
		writeFileSync(f.metadata, JSON.stringify(record))
		await expect(fixture(f.dataDir).vault.state()).rejects.toThrow(/invalid desktop security file/)
		writeFileSync(f.metadata, "{not json")
		await expect(fixture(f.dataDir).vault.state()).rejects.toThrow(/invalid desktop security file/)
	})

	it("recovers with previously enrolled Touch ID without asking for the forgotten password", async () => {
		const f = fixture()
		f.biometrics.available = vi.fn(async () => true)
		await enroll(f, true)
		const key = Buffer.from(f.start.mock.calls[0][0])
		const calls: string[] = []
		const next = fixture(f.dataDir, {
			biometrics: {
				...f.biometrics,
				confirm: async () => {
					calls.push("confirm")
				},
				unprotect: async () => {
					calls.push("unprotect")
					return key
				},
			},
		})
		await next.vault.recover({ method: "biometric" })
		expect(calls).toEqual(["confirm", "unprotect"])
		expect(next.start).not.toHaveBeenCalled()
		await next.vault.resetPassword(replacement)
		await next.vault.confirmRecovery(true)
		await fixture(f.dataDir).unlock({ method: "password", password: replacement.password })
	})

	it("keeps an acknowledged password reset retryable if the solver fails to start", async () => {
		const f = fixture()
		const code = await enroll(f)
		const next = fixture(f.dataDir)
		await next.vault.recover({ method: "code", recoveryCode: code })
		await next.vault.resetPassword(replacement)
		const replacementCode = next.vault.recoveryCode()
		next.start.mockRejectedValueOnce(new Error("boot failed"))
		await expect(next.vault.confirmRecovery(true)).rejects.toThrow("boot failed")
		expect(next.vault.isUnlocked()).toBe(false)
		expect(next.vault.recoveryCode()).toBe(replacementCode)
		await next.vault.confirmRecovery(true)
		expect(next.vault.isUnlocked()).toBe(true)
		await fixture(f.dataDir).unlock({ method: "password", password: replacement.password })
	})

	it("rejects stale reset sessions rather than overwriting newer security metadata", async () => {
		const f = fixture()
		const code = await enroll(f)
		const next = fixture(f.dataDir)
		await next.vault.recover({ method: "code", recoveryCode: code })
		await next.vault.resetPassword(replacement)
		const changed = readFileSync(f.metadata, "utf8") + "\n"
		writeFileSync(f.metadata, changed)
		await expect(next.vault.confirmRecovery(true)).rejects.toThrow(/Security settings changed/)
		expect(readFileSync(f.metadata, "utf8")).toBe(changed)
		expect(next.start).not.toHaveBeenCalled()
	})

	it("protects every recovery mutation with CSRF checks and exposes codes only while awaiting acknowledgement", async () => {
		const f = fixture()
		for (const route of ["recover", "reset-password", "confirm-recovery", "cancel-recovery"]) {
			const response = await f.vault.handle(
				new Request(`simplex://local/api/desktop/${route}`, { method: "POST", body: "{}" }),
			)
			expect(response.status).toBe(403)
		}
		const request = () =>
			new Request("simplex://local/api/desktop/recovery-code", { headers: { "X-Simplex-UI": "1" } })
		expect((await f.vault.handle(request())).status).toBe(400)
		await f.vault.unlock(create)
		expect((await f.vault.handle(new Request("simplex://local/api/desktop/recovery-code"))).status).toBe(403)
		expect(
			(
				await f.vault.handle(
					new Request("simplex://local/api/desktop/recovery-code", {
						headers: { "X-Simplex-UI": "1", Origin: "https://evil.example" },
					}),
				)
			).status,
		).toBe(403)
		const response = await f.vault.handle(request())
		expect(response.headers.get("cache-control")).toBe("no-store")
		expect((await response.json()).code).toBe(f.vault.recoveryCode())
		await f.vault.confirmRecovery(true)
		expect((await f.vault.handle(request())).status).toBe(400)
	})

	it("validates password confirmation and serializes expensive unlock attempts", async () => {
		for (const request of [
			{ ...create, password: "short" },
			{ ...create, confirmation: "different" },
		]) {
			const f = fixture()
			await expect(f.vault.unlock(request)).rejects.toThrow()
			expect(f.prepare).not.toHaveBeenCalled()
			expect(readdirSync(f.dataDir)).toEqual([])
		}
		const f = fixture()
		const first = f.unlock(create)
		await expect(f.vault.unlock(create)).rejects.toThrow(/wait a moment/)
		await first
		expect(f.start).toHaveBeenCalledOnce()
	})

	it("migrates the final legacy config atomically and starts locked on the next launch", async () => {
		const f = fixture()
		writeFileSync(f.path, "legacy wallet secret")
		f.prepare.mockImplementation(async () => {
			writeFileSync(f.path, "final wallet secret")
		})
		await f.unlock(create)
		const key = f.start.mock.calls[0][0]
		expect(f.vault.isUnlocked()).toBe(true)
		expect(decryptConfig(readFileSync(f.path, "utf8"), key)).toBe("final wallet secret")
		expect(readFileSync(f.metadata, "utf8")).not.toContain(password)
		expect(readFileSync(f.metadata, "utf8")).not.toContain(key.toString("hex"))
		expect(readdirSync(f.dataDir).sort()).toEqual(["desktop-vault.json", "filler-config.toml"])
		const next = fixture(f.dataDir)
		expect((await next.vault.state()).mode).toBe("unlock")
		expect(next.start).not.toHaveBeenCalled()
		await next.unlock({ method: "password", password })
		expect(next.start.mock.calls[0][0]).toEqual(key)
		f.vault.lock()
		expect(key).toEqual(Buffer.alloc(32))
	})

	it("encrypts first-run setup and subsequent edits without a plaintext intermediate", async () => {
		const f = fixture()
		f.start.mockImplementation(async (key) => {
			const store = encryptedConfigStore(f.path, key)
			store.write(f.path, "initial credentials")
			store.write(f.path, "changed credentials")
		})
		await f.unlock(create)
		expect(isEncryptedConfig(readFileSync(f.path, "utf8"))).toBe(true)
		expect(readFileSync(f.path, "utf8")).not.toContain("credentials")
	})

	it("rejects a wrong password and damaged ciphertext before touching the solver", async () => {
		const f = fixture()
		writeFileSync(f.path, "wallet secret")
		await f.unlock(create)
		const before = readFileSync(f.path, "utf8")
		const wrong = fixture(f.dataDir)
		await expect(wrong.vault.unlock({ method: "password", password: "incorrect" })).rejects.toThrow(
			/Incorrect password/,
		)
		expect(wrong.prepare).not.toHaveBeenCalled()
		expect(wrong.start).not.toHaveBeenCalled()
		expect(wrong.vault.isUnlocked()).toBe(false)
		expect(readFileSync(f.path, "utf8")).toBe(before)
		writeFileSync(f.path, before.replace(/"tag":"[a-f0-9]+"/, `"tag":"${"00".repeat(16)}"`))
		const corrupt = fixture(f.dataDir)
		await expect(corrupt.vault.unlock({ method: "password", password })).rejects.toThrow()
		expect(corrupt.prepare).not.toHaveBeenCalled()
	})

	it("recovers after interrupted migration or failed boot using the persisted wrapped key", async () => {
		const f = fixture()
		writeFileSync(f.path, "wallet secret")
		f.start.mockRejectedValue(new Error("boot failed"))
		await expect(f.unlock(create)).rejects.toThrow("boot failed")
		expect(f.vault.isUnlocked()).toBe(false)
		writeFileSync(f.path, "legacy config from interrupted migration")
		const retry = fixture(f.dataDir)
		await retry.unlock({ method: "password", password })
		expect(isEncryptedConfig(readFileSync(f.path, "utf8"))).toBe(true)
		expect(retry.vault.isUnlocked()).toBe(true)
		unlinkSync(f.metadata)
		await expect(fixture(f.dataDir).vault.unlock(create)).rejects.toThrow(/security file is missing/)
	})

	it("keeps password fallback when Touch ID is unavailable or cancelled", async () => {
		const f = fixture()
		f.biometrics.available = vi.fn(async () => true)
		await f.unlock({ ...create, useBiometrics: true })
		expect(f.biometrics.confirm).toHaveBeenCalledOnce()
		const biometrics = {
			...f.biometrics,
			confirm: vi.fn(async () => {
				throw new Error("cancelled")
			}),
		}
		const cancelled = fixture(f.dataDir, { biometrics })
		await expect(cancelled.vault.unlock({ method: "biometric" })).rejects.toThrow("cancelled")
		expect(biometrics.unprotect).not.toHaveBeenCalled()
		expect(cancelled.start).not.toHaveBeenCalled()
		const unavailable = fixture(f.dataDir)
		await unavailable.unlock({ method: "password", password })
		expect(unavailable.vault.isUnlocked()).toBe(true)
	})

	it("does not migrate before the running solver is safely stopped", async () => {
		const f = fixture(undefined, {
			prepare: async (allowed) => {
				if (!allowed) throw new Error("restart required")
			},
			needsRestart: async () => true,
		})
		writeFileSync(f.path, "wallet secret")
		expect((await f.vault.state()).needsRestart).toBe(true)
		await expect(f.unlock(create)).rejects.toThrow("restart required")
		expect(readFileSync(f.path, "utf8")).toBe("wallet secret")
		expect(readdirSync(f.dataDir)).toEqual(["filler-config.toml"])
		expect(f.start).not.toHaveBeenCalled()
		const next = fixture(f.dataDir)
		await next.unlock({ ...create, restartSolver: true })
		expect(next.prepare).toHaveBeenCalledWith(true)
		expect(isEncryptedConfig(readFileSync(f.path, "utf8"))).toBe(true)
	})

	it("requires Touch ID confirmation before releasing the stored key", async () => {
		const f = fixture()
		f.biometrics.available = vi.fn(async () => true)
		await f.unlock({ ...create, useBiometrics: true })
		const key = Buffer.from(f.start.mock.calls[0][0])
		const calls: string[] = []
		const next = fixture(f.dataDir, {
			biometrics: {
				...f.biometrics,
				confirm: async () => {
					calls.push("confirm")
				},
				unprotect: async () => {
					calls.push("unprotect")
					return key
				},
			},
		})
		await next.unlock({ method: "biometric" })
		expect(calls).toEqual(["confirm", "unprotect"])
		expect(next.start).toHaveBeenCalledWith(key)
	})

	it("requires CSRF protection and bounds unlock request bodies", async () => {
		const f = fixture()
		const request = (body: string, headers: Record<string, string> = {}) =>
			new Request("simplex://local/api/desktop/unlock", { method: "POST", headers, body })
		expect((await f.vault.handle(request(JSON.stringify(create)))).status).toBe(403)
		expect((await f.vault.handle(request(" ".repeat(16_385), { "x-simplex-ui": "1" }))).status).toBe(413)
		const malformed = await f.vault.handle(
			request('{"password":"do-not-echo-this-secret"', { "x-simplex-ui": "1" }),
		)
		expect(malformed.status).toBe(400)
		expect(await malformed.text()).not.toContain("do-not-echo-this-secret")
		expect(
			(
				await f.vault.handle(
					request(JSON.stringify(create), { "x-simplex-ui": "1", origin: "https://example.com" }),
				)
			).status,
		).toBe(403)
		expect(f.start).not.toHaveBeenCalled()
	})
})
