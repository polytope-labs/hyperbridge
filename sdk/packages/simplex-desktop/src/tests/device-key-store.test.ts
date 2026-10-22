import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { safeStorage } from "electron"
import { createDeviceKeyStore } from "../security/device-key-store"

vi.mock("electron", () => ({
	safeStorage: {
		isAsyncEncryptionAvailable: vi.fn(),
		getSelectedStorageBackend: vi.fn(),
		encryptStringAsync: vi.fn(),
		decryptStringAsync: vi.fn(),
	},
}))

beforeEach(() => {
	vi.resetAllMocks()
	vi.mocked(safeStorage.isAsyncEncryptionAvailable).mockResolvedValue(true)
})
afterEach(() => vi.useRealTimers())

describe("OS-protected background solver key", () => {
	it.each(["darwin", "win32", "linux"] as const)("bounds a stalled %s check and allows a later recovery", async (platform) => {
		vi.useFakeTimers()
		let complete!: (available: boolean) => void
		vi.mocked(safeStorage.isAsyncEncryptionAvailable).mockReturnValueOnce(
			new Promise((resolve) => {
				complete = resolve
			}),
		)
		vi.mocked(safeStorage.getSelectedStorageBackend).mockReturnValue("gnome_libsecret")
		const store = createDeviceKeyStore(platform)
		const results: boolean[] = []
		void store.available().then((result) => results.push(result))
		void store.available().then((result) => results.push(result))
		await vi.advanceTimersByTimeAsync(5_000)
		expect(results).toEqual([false, false])
		expect(await store.available()).toBe(false)
		expect(safeStorage.isAsyncEncryptionAvailable).toHaveBeenCalledOnce()
		await expect(store.protect(Buffer.alloc(32))).rejects.toThrow(/secure OS key store/)
		expect(safeStorage.encryptStringAsync).not.toHaveBeenCalled()
		complete(true)
		await vi.advanceTimersByTimeAsync(0)
		expect(await store.available()).toBe(true)
	})

	it.each(["basic_text", "unknown"] as const)("rejects insecure Linux backend %s", async (backend) => {
		vi.mocked(safeStorage.getSelectedStorageBackend).mockReturnValue(backend)
		const store = createDeviceKeyStore("linux")
		expect(await store.available()).toBe(false)
		await expect(store.protect(Buffer.alloc(32))).rejects.toThrow(/secure OS key store/)
		await expect(store.unprotect("abcd")).rejects.toThrow(/secure OS key store/)
		expect(safeStorage.encryptStringAsync).not.toHaveBeenCalled()
		expect(safeStorage.decryptStringAsync).not.toHaveBeenCalled()
	})

	it.each(["darwin", "win32", "linux"] as const)("uses secure storage on %s", async (platform) => {
		vi.mocked(safeStorage.getSelectedStorageBackend).mockReturnValue("gnome_libsecret")
		vi.mocked(safeStorage.encryptStringAsync).mockResolvedValue(Buffer.from("sealed"))
		vi.mocked(safeStorage.decryptStringAsync).mockResolvedValue({
			result: "ab".repeat(32),
			shouldReEncrypt: false,
		})
		const store = createDeviceKeyStore(platform)
		expect(await store.available()).toBe(true)
		expect(await store.protect(Buffer.alloc(32, 0xab))).toBe(Buffer.from("sealed").toString("hex"))
		expect(await store.unprotect(Buffer.from("sealed").toString("hex"))).toEqual(Buffer.alloc(32, 0xab))
	})

	it("rejects malformed decrypted key material", async () => {
		vi.mocked(safeStorage.decryptStringAsync).mockResolvedValue({ result: "short", shouldReEncrypt: false })
		await expect(createDeviceKeyStore("darwin").unprotect("aabb")).rejects.toThrow(/invalid/)
	})
})
