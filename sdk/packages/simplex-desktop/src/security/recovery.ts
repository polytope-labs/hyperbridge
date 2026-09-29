import { randomBytes } from "node:crypto"
import { openSecret, sealSecret, type SealedSecret } from "@hyperbridge/simplex/config-storage"

const PURPOSE = "simplex/recovery/v1"

/** A random 256-bit recovery code wraps the config key, never the password. */
export function createRecoveryKey(configKey: Buffer): { code: string; wrappedKey: SealedSecret } {
	const recoveryKey = randomBytes(32)
	try {
		return {
			code: recoveryKey.toString("hex").toUpperCase().match(/.{8}/g)!.join("-"),
			wrappedKey: sealSecret(configKey, recoveryKey, PURPOSE),
		}
	} finally {
		recoveryKey.fill(0)
	}
}

export function recoverConfigKey(code: unknown, wrappedKey: SealedSecret): Buffer {
	if (typeof code !== "string" || code.length > 256) throw new Error("Invalid recovery code")
	const normalized = code.replace(/[\s-]/g, "").toLowerCase()
	if (!/^[a-f0-9]{64}$/.test(normalized)) throw new Error("Invalid recovery code")
	const recoveryKey = Buffer.from(normalized, "hex")
	try {
		return openSecret(wrappedKey, recoveryKey, PURPOSE)
	} catch {
		throw new Error("Invalid recovery code or damaged security file")
	} finally {
		recoveryKey.fill(0)
	}
}
