import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import { join } from "node:path"
import { decryptConfig, isEncryptedConfig, openSecret, passwordKey } from "../../../simplex/src/config/storage.ts"

export const TEST_PASSWORD = "simplex-test-password-only"

/** Exercise the actual login form; no production authentication bypass. */
export async function unlockDesktop(page, { pending = false, restartSolver = false } = {}) {
	let recoveryCode
	await page.locator("#desktop-password").waitFor()
	const creating = (await page.locator("#desktop-confirmation").count()) > 0
	await page.locator("#desktop-password").fill(TEST_PASSWORD)
	if (creating) await page.locator("#desktop-confirmation").fill(TEST_PASSWORD)
	if (restartSolver) await page.getByRole("checkbox", { name: /Stop and restart the running solver/ }).check()
	await page.getByRole("button", { name: creating ? "Continue" : "Unlock", exact: true }).click()
	if (creating) {
		await page.locator("#saved-recovery-code").waitFor()
		recoveryCode = await page.locator("#saved-recovery-code").textContent()
		await page.getByRole("checkbox", { name: "I've saved my recovery code" }).check()
		await page.getByRole("button", { name: "Continue", exact: true }).click()
	}
	if (!pending) await page.locator(".desktop-unlock").waitFor({ state: "detached", timeout: 120_000 })
	return recoveryCode
}

export async function decryptedTestConfig(userData) {
	const ciphertext = await readFile(join(userData, "filler-config.toml"), "utf8")
	assert.ok(isEncryptedConfig(ciphertext), "desktop config must be encrypted on disk")
	const record = JSON.parse(await readFile(join(userData, "desktop-vault.json"), "utf8"))
	const wrappingKey = await passwordKey(TEST_PASSWORD, record.salt)
	const key = openSecret(record.wrappedKey, wrappingKey, "simplex/password/v1")
	try {
		return decryptConfig(ciphertext, key)
	} finally {
		wrappingKey.fill(0)
		key.fill(0)
	}
}
