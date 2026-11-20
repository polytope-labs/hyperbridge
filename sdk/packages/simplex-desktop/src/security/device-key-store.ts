import { safeStorage } from "electron"

/** OS-user protected copy used only to resume the solver while the UI remains locked. */
export interface DeviceKeyStore {
	available(): Promise<boolean>
	protect(key: Buffer): Promise<string>
	unprotect(value: string): Promise<Buffer>
}

export function createDeviceKeyStore(platform: NodeJS.Platform = process.platform): DeviceKeyStore {
	let availability: Promise<boolean> | undefined
	async function probeAvailability(): Promise<boolean> {
		try {
			if (!(await safeStorage.isAsyncEncryptionAvailable())) return false
			if (platform === "linux") {
				const backend = safeStorage.getSelectedStorageBackend()
				return backend !== "basic_text" && backend !== "unknown"
			}
			return platform === "darwin" || platform === "win32"
		} catch {
			return false
		}
	}
	return {
		available() {
			availability ??= new Promise<boolean>((resolve) => {
				// OS storage can remain pending indefinitely. Password login must still
				// work; retain the shared result until the native call finishes so retries
				// cannot accumulate unresolved native checks or promise callbacks.
				const timer = setTimeout(() => resolve(false), 5_000)
				void probeAvailability().then((available) => {
					clearTimeout(timer)
					availability = undefined
					resolve(available)
				})
			})
			return availability
		},
		async protect(key) {
			if (key.length !== 32 || !(await this.available())) throw new Error("A secure OS key store is unavailable")
			return (await safeStorage.encryptStringAsync(key.toString("hex"))).toString("hex")
		},
		async unprotect(value) {
			if (!/^(?:[a-f0-9]{2})+$/.test(value) || !(await this.available()))
				throw new Error("A secure OS key store is unavailable")
			const { result } = await safeStorage.decryptStringAsync(Buffer.from(value, "hex"))
			if (!/^[a-f0-9]{64}$/.test(result)) throw new Error("The saved OS key is invalid")
			return Buffer.from(result, "hex")
		},
	}
}

export const deviceKeyStore = createDeviceKeyStore()
