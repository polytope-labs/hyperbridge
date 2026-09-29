import { beforeEach, describe, expect, it, vi } from "vitest"
import { safeStorage, systemPreferences } from "electron"
import { createTouchIdUnlock } from "../security/biometrics"

vi.mock("electron", () => ({
	systemPreferences: { canPromptTouchID: vi.fn(), promptTouchID: vi.fn() },
	safeStorage: { isAsyncEncryptionAvailable: vi.fn(), encryptStringAsync: vi.fn(), decryptStringAsync: vi.fn() },
}))
beforeEach(() => {
	vi.resetAllMocks()
})

describe("platform biometric availability", () => {
	it.each(["win32", "linux"] as const)("does not expose Touch ID or invoke platform APIs on %s", async (platform) => {
		const biometrics = createTouchIdUnlock(platform)
		expect(await biometrics.available()).toBe(false)
		expect(() => biometrics.confirm()).toThrow(/supported Macs/)
		await expect(biometrics.protect(Buffer.alloc(32))).rejects.toThrow(/supported Macs/)
		await expect(biometrics.unprotect("aabb")).rejects.toThrow(/supported Macs/)
		expect(systemPreferences.canPromptTouchID).not.toHaveBeenCalled()
		expect(systemPreferences.promptTouchID).not.toHaveBeenCalled()
		expect(safeStorage.isAsyncEncryptionAvailable).not.toHaveBeenCalled()
	})
	it("requires usable Touch ID and secure storage on macOS", async () => {
		const biometrics = createTouchIdUnlock("darwin")
		vi.mocked(systemPreferences.canPromptTouchID).mockReturnValue(false)
		expect(await biometrics.available()).toBe(false)
		vi.mocked(systemPreferences.canPromptTouchID).mockReturnValue(true)
		vi.mocked(safeStorage.isAsyncEncryptionAvailable).mockResolvedValue(false)
		expect(await biometrics.available()).toBe(false)
		vi.mocked(safeStorage.isAsyncEncryptionAvailable).mockResolvedValue(true)
		expect(await biometrics.available()).toBe(true)
		vi.mocked(safeStorage.isAsyncEncryptionAvailable).mockRejectedValue(new Error("Keychain locked"))
		expect(await biometrics.available()).toBe(false)
	})
})
