import { beforeEach, describe, expect, it, vi } from "vitest"
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

describe("OS-protected background solver key", () => {
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
