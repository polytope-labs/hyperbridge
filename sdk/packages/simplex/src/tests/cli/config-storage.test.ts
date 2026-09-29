import { randomBytes } from "node:crypto"
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { PassThrough } from "node:stream"
import { afterEach, describe, expect, it } from "vitest"
import { decryptConfig, encryptedConfigStore, encryptConfig, readConfigFile, readConfigKey } from "@/config/storage"

const directories: string[] = []
afterEach(() => {
	for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
})

describe("encrypted config storage", () => {
	it("authenticates ciphertext and uses a new nonce on each save", () => {
		const key = randomBytes(32)
		const content = "[simplex.signer]\nprivateKey = 'secret-wallet-key'\n"
		const encrypted = encryptConfig(content, key)
		expect(encrypted).not.toContain("secret-wallet-key")
		expect(encrypted).not.toEqual(encryptConfig(content, key))
		expect(decryptConfig(encrypted, key)).toBe(content)
		expect(() => decryptConfig(encrypted, randomBytes(32))).toThrow()
		const [header, json] = encrypted.split("\n")
		const payload = JSON.parse(json)
		payload.tag = "00".repeat(16)
		expect(() => decryptConfig(`${header}\n${JSON.stringify(payload)}`, key)).toThrow()
		expect(() => decryptConfig(encrypted.replace("V1", "V2"), key)).toThrow(/Unsupported/)
	})

	it("encrypts initial and subsequent writes, restricts destinations, and requires unlock on read", () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-config-security-"))
		directories.push(directory)
		const path = join(directory, "filler-config.toml")
		const store = encryptedConfigStore(path, randomBytes(32))
		expect(store.exists()).toBe(false)
		store.write(path, "first secret")
		expect(store.read()).toBe("first secret")
		store.write(path, "updated secret")
		expect(store.read()).toBe("updated secret")
		expect(readFileSync(path, "utf8")).not.toContain("secret")
		expect(() => readConfigFile(path)).toThrow(/Desktop to unlock/)
		expect(() => store.write(join(directory, "leak.toml"), "secret")).toThrow(/protected data directory/)
		expect(readdirSync(directory)).toEqual(["filler-config.toml"])
		if (process.platform !== "win32") expect(statSync(path).mode & 0o777).toBe(0o600)
	})

	it("reads exactly one key from the inherited pipe and rejects truncated or oversized input", async () => {
		const key = randomBytes(32)
		const input = new PassThrough()
		const received = readConfigKey(input)
		input.end(Buffer.from(key))
		expect(await received).toEqual(key)
		for (const length of [0, 31, 33]) {
			const bad = new PassThrough()
			const result = readConfigKey(bad)
			bad.end(Buffer.alloc(length))
			await expect(result).rejects.toThrow(/handoff/)
		}
		await expect(readConfigKey(new PassThrough(), 5)).rejects.toThrow(/timed out/)
	})
})
