import { createCipheriv, createDecipheriv, randomBytes, scrypt } from "node:crypto"
import {
	closeSync,
	existsSync,
	fsyncSync,
	openSync,
	readFileSync,
	renameSync,
	unlinkSync,
	writeFileSync,
} from "node:fs"
import { basename, dirname, join, resolve } from "node:path"
import type { Readable } from "node:stream"

const CONFIG_HEADER = "SIMPLEX-ENCRYPTED-CONFIG-V1\n"
const MAX_CONFIG_BYTES = 2 * 1024 * 1024
export type SealedSecret = { iv: string; tag: string; data: string }

/** Used for both the encrypted config and its wrapped key; no plaintext temp file is created. */
export function writeSecretAtomic(path: string, content: string): void {
	const temp = join(dirname(path), `.${basename(path)}.${randomBytes(12).toString("hex")}.tmp`)
	let fd: number | undefined
	try {
		fd = openSync(temp, "wx", 0o600)
		writeFileSync(fd, content)
		fsyncSync(fd)
		closeSync(fd)
		fd = undefined
		renameSync(temp, path)
		// Persist the directory entry before a following encrypted write relies on
		// this file (notably the password-wrapped key written before migration).
		if (process.platform !== "win32") {
			const directory = openSync(dirname(path), "r")
			try {
				fsyncSync(directory)
			} finally {
				closeSync(directory)
			}
		}
	} catch (error) {
		if (fd !== undefined) closeSync(fd)
		try {
			unlinkSync(temp)
		} catch {}
		throw error
	}
}

function bytes(value: unknown, length?: number): Buffer {
	if (typeof value !== "string" || value.length > MAX_CONFIG_BYTES * 2 || !/^(?:[a-f0-9]{2})*$/.test(value)) {
		throw new Error("Invalid encrypted data")
	}
	const decoded = Buffer.from(value, "hex")
	if (length !== undefined && decoded.length !== length) throw new Error("Invalid encrypted data")
	return decoded
}

export function sealSecret(plaintext: Buffer, key: Buffer, purpose: string): SealedSecret {
	if (key.length !== 32) throw new Error("Invalid encryption key")
	const iv = randomBytes(12)
	const cipher = createCipheriv("aes-256-gcm", key, iv)
	const aad = Buffer.from(purpose)
	cipher.setAAD(aad)
	const data = Buffer.concat([cipher.update(plaintext), cipher.final()])
	return { iv: iv.toString("hex"), tag: cipher.getAuthTag().toString("hex"), data: data.toString("hex") }
}

export function openSecret(sealed: SealedSecret, key: Buffer, purpose: string): Buffer {
	if (key.length !== 32 || !sealed || typeof sealed !== "object") throw new Error("Invalid encrypted data")
	const decipher = createDecipheriv("aes-256-gcm", key, bytes(sealed.iv, 12))
	decipher.setAAD(Buffer.from(purpose))
	decipher.setAuthTag(bytes(sealed.tag, 16))
	return Buffer.concat([decipher.update(bytes(sealed.data)), decipher.final()])
}

/** Fixed work factors are versioned by the desktop vault, never supplied by an untrusted file. */
export function passwordKey(password: string, salt: string): Promise<Buffer> {
	const saltBytes = bytes(salt, 16)
	return new Promise((resolveKey, reject) => {
		scrypt(password, saltBytes, 32, { N: 131072, r: 8, p: 1, maxmem: 192 * 1024 * 1024 }, (error, key) => {
			if (error) reject(error)
			else resolveKey(key)
		})
	})
}

export function isEncryptedConfig(content: string): boolean {
	return content.startsWith("SIMPLEX-ENCRYPTED-CONFIG-")
}

export function encryptConfig(content: string, key: Buffer): string {
	if (Buffer.byteLength(content) > MAX_CONFIG_BYTES) throw new Error("Config is too large")
	const plaintext = Buffer.from(content)
	try {
		return CONFIG_HEADER + JSON.stringify(sealSecret(plaintext, key, "simplex/config/v1")) + "\n"
	} finally {
		plaintext.fill(0)
	}
}

export function decryptConfig(content: string, key: Buffer): string {
	if (!content.startsWith(CONFIG_HEADER) || content.length > MAX_CONFIG_BYTES * 2 + 1024) {
		throw new Error("Unsupported or invalid encrypted config")
	}
	const plaintext = openSecret(JSON.parse(content.slice(CONFIG_HEADER.length)), key, "simplex/config/v1")
	try {
		return plaintext.toString("utf8")
	} finally {
		plaintext.fill(0)
	}
}

export function readConfigFile(path: string, key?: Buffer): string {
	const content = readFileSync(path, "utf8")
	if (isEncryptedConfig(content)) {
		if (!key) throw new Error("This config is encrypted. Open Simplex Desktop to unlock it.")
		return decryptConfig(content, key)
	}
	if (key) throw new Error("Refusing to load a plaintext config in protected desktop mode")
	return content
}

/** One protected destination for initial setup and every subsequent runtime edit. */
export function encryptedConfigStore(path: string, key: Buffer) {
	const target = resolve(path)
	return {
		path: target,
		exists: () => existsSync(target),
		read: () => readConfigFile(target, key),
		write: (requestedPath: string, content: string) => {
			if (resolve(requestedPath) !== target)
				throw new Error("Desktop config must stay in its protected data directory")
			writeSecretAtomic(target, encryptConfig(content, key))
		},
	}
}

/** A one-shot inherited pipe avoids keys in argv, the environment, or temporary files. */
export function readConfigKey(input: Readable, timeoutMs = 5_000): Promise<Buffer> {
	return new Promise((resolveKey, reject) => {
		const key = Buffer.alloc(32)
		let offset = 0
		const finish = (error?: Error) => {
			clearTimeout(timer)
			input.removeListener("data", onData)
			input.removeListener("end", onEnd)
			input.removeListener("error", onError)
			input.destroy()
			if (error) {
				key.fill(0)
				reject(error)
			} else resolveKey(key)
		}
		const onData = (chunk: Buffer) => {
			if (!Buffer.isBuffer(chunk) || offset + chunk.length > 32)
				return finish(new Error("Invalid desktop key handoff"))
			chunk.copy(key, offset)
			offset += chunk.length
			chunk.fill(0)
		}
		const onEnd = () => finish(offset === 32 ? undefined : new Error("Incomplete desktop key handoff"))
		const onError = () => finish(new Error("Desktop key handoff failed"))
		const timer = setTimeout(() => finish(new Error("Desktop key handoff timed out")), timeoutMs)
		input.on("data", onData).once("end", onEnd).once("error", onError)
	})
}
