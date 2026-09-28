import { safeStorage, systemPreferences } from "electron"
import type { BiometricUnlock } from "./vault"

/** Touch ID authorizes app access; macOS Keychain protects the optional wrapped key at rest. */
export function createTouchIdUnlock(platform: NodeJS.Platform = process.platform): BiometricUnlock {
	const requireMac = () => {
		if (platform !== "darwin") throw new Error("Touch ID is only available on supported Macs")
	}
	return {
		async available() {
			try {
				return (
					platform === "darwin" &&
					systemPreferences.canPromptTouchID() &&
					(await safeStorage.isAsyncEncryptionAvailable())
				)
			} catch {
				// An unavailable Keychain must not block password recovery.
				return false
			}
		},
		confirm: () => {
			requireMac()
			return systemPreferences.promptTouchID("unlock your Simplex configuration")
		},
		async protect(key) {
			requireMac()
			return (await safeStorage.encryptStringAsync(key.toString("hex"))).toString("hex")
		},
		async unprotect(value) {
			requireMac()
			const { result } = await safeStorage.decryptStringAsync(Buffer.from(value, "hex"))
			if (!/^[a-f0-9]{64}$/.test(result)) throw new Error("Touch ID key is unavailable. Use your password.")
			return Buffer.from(result, "hex")
		},
	}
}

export const touchIdUnlock = createTouchIdUnlock()
