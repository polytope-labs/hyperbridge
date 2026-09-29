import { randomBytes } from "node:crypto"
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs"
import { join } from "node:path"
import {
	decryptConfig,
	encryptConfig,
	isEncryptedConfig,
	openSecret,
	passwordKey,
	sealSecret,
	writeSecretAtomic,
	type SealedSecret,
} from "@hyperbridge/simplex/config-storage"
import { createRecoveryKey, recoverConfigKey } from "./recovery"
import type { DeviceKeyStore } from "./device-key-store"

type VaultRecord = {
	version: 2
	salt: string
	wrappedKey: SealedSecret
	biometricKey?: string
	recoveryKey: SealedSecret
	deviceKey?: string
}
/** Credential wrappers before a recovery code is attached; never persisted in this shape. */
type Credentials = Omit<VaultRecord, "version" | "recoveryKey">
export interface BiometricUnlock {
	available(): Promise<boolean>
	confirm(): Promise<void>
	protect(key: Buffer): Promise<string>
	unprotect(value: string): Promise<Buffer>
}
export interface DesktopAccessState {
	mode: "create" | "unlock" | "reset-password" | "save-recovery" | "unlocked"
	biometricAvailable: boolean
	biometricEnabled: boolean
	recoveryEnabled: boolean
	backgroundResumeAvailable: boolean
	secureDeviceStorageAvailable: boolean
	needsRestart: boolean
}
type UnlockRequest = {
	method?: unknown
	password?: unknown
	confirmation?: unknown
	useBiometrics?: unknown
	restartSolver?: unknown
	recoveryCode?: unknown
}
type AuthorizedRecovery = { key: Buffer; record: VaultRecord; revision: string | undefined; expiresAt: number }
type PendingSave = AuthorizedRecovery & { code: string; restartAllowed: boolean; committed: boolean }
const RECOVERY_SESSION_MS = 10 * 60_000

/** Authentication lives in the host, including recovery and its confirmation boundary. */
export class DesktopVault {
	private readonly vaultPath: string
	private readonly configPath: string
	private key?: Buffer
	private recovery?: AuthorizedRecovery
	private pending?: PendingSave
	private expiryTimer?: NodeJS.Timeout
	private busy = false
	private nextAttempt = 0
	constructor(
		private readonly options: {
			dataDir: string
			biometrics: BiometricUnlock
			deviceKeyStore: DeviceKeyStore
			needsRestart(): Promise<boolean>
			prepare(restartAllowed: boolean): Promise<void>
			start(key: Buffer): Promise<void>
			onUnlocked?(): void
		},
	) {
		this.vaultPath = join(options.dataDir, "desktop-vault.json")
		this.configPath = join(options.dataDir, "filler-config.toml")
	}
	isUnlocked(): boolean {
		return this.key !== undefined
	}
	lock(): void {
		clearTimeout(this.expiryTimer)
		this.key?.fill(0)
		this.recovery?.key.fill(0)
		this.pending?.key.fill(0)
		this.key = undefined
		this.recovery = undefined
		this.pending = undefined
	}
	private expire(): void {
		if (this.busy) return
		if (this.recovery && Date.now() >= this.recovery.expiresAt) {
			this.recovery.key.fill(0)
			this.recovery = undefined
		}
		if (this.pending && Date.now() >= this.pending.expiresAt) {
			this.pending.key.fill(0)
			this.pending = undefined
		}
	}
	/** Zero recovery keys when their session lapses, not only on the next request. */
	private scheduleExpiry(): void {
		clearTimeout(this.expiryTimer)
		this.expiryTimer = setTimeout(() => this.expire(), RECOVERY_SESSION_MS + 1)
		this.expiryTimer.unref()
	}
	private revision(): string | undefined {
		if (!existsSync(this.vaultPath)) return undefined
		if (statSync(this.vaultPath).size > 16_384) throw new Error("Invalid desktop security file")
		return readFileSync(this.vaultPath, "utf8")
	}
	private record(): VaultRecord | undefined {
		const content = this.revision()
		if (content === undefined) {
			if (existsSync(this.configPath) && isEncryptedConfig(readFileSync(this.configPath, "utf8")))
				throw new Error("The security file is missing. Restore desktop-vault.json from your backup.")
			return undefined
		}
		let record: VaultRecord
		try {
			record = JSON.parse(content) as VaultRecord
		} catch {
			throw new Error("Unsupported or invalid desktop security file")
		}
		if (
			!record ||
			record.version !== 2 ||
			!/^[a-f0-9]{32}$/.test(record.salt) ||
			!record.wrappedKey ||
			!record.recoveryKey ||
			(record.deviceKey !== undefined &&
				(typeof record.deviceKey !== "string" || !/^(?:[a-f0-9]{2})+$/.test(record.deviceKey))) ||
			(record.biometricKey !== undefined &&
				(typeof record.biometricKey !== "string" || !/^(?:[a-f0-9]{2})+$/.test(record.biometricKey)))
		)
			throw new Error("Unsupported or invalid desktop security file")
		return record
	}
	async state(): Promise<DesktopAccessState> {
		this.expire()
		const record = this.record()
		const secureDeviceStorageAvailable = await this.options.deviceKeyStore.available()
		return {
			mode: this.key
				? "unlocked"
				: this.pending
					? "save-recovery"
					: this.recovery
						? "reset-password"
						: record
							? "unlock"
							: "create",
			biometricAvailable: await this.options.biometrics.available(),
			biometricEnabled: Boolean(record?.biometricKey),
			recoveryEnabled: Boolean(record),
			backgroundResumeAvailable:
				Boolean(this.pending?.record.deviceKey ?? record?.deviceKey) && secureDeviceStorageAvailable,
			secureDeviceStorageAvailable,
			needsRestart: !this.key && (await this.options.needsRestart()),
		}
	}
	private async exclusive(action: () => Promise<void>, authentication = false): Promise<void> {
		this.expire()
		if (this.busy || (authentication && Date.now() < this.nextAttempt))
			throw new Error("Please wait a moment before trying again")
		this.busy = true
		try {
			await action()
		} finally {
			this.busy = false
			if (authentication) this.nextAttempt = Date.now() + 1_000
		}
	}
	private checkConfig(key: Buffer): void {
		if (key.length !== 32) throw new Error("Invalid desktop encryption key")
		if (existsSync(this.configPath)) {
			const content = readFileSync(this.configPath, "utf8")
			if (isEncryptedConfig(content)) decryptConfig(content, key)
		}
	}
	private async biometricKey(record: VaultRecord): Promise<Buffer> {
		if (!record.biometricKey || !(await this.options.biometrics.available()))
			throw new Error("Touch ID is unavailable. Use your password or recovery code.")
		await this.options.biometrics.confirm()
		return this.options.biometrics.unprotect(record.biometricKey)
	}
	private async enrollBiometrics(key: Buffer): Promise<string> {
		if (!(await this.options.biometrics.available())) throw new Error("Touch ID is unavailable on this device")
		await this.options.biometrics.confirm()
		return this.options.biometrics.protect(key)
	}
	private async wrapDeviceKey(key: Buffer): Promise<string | undefined> {
		try {
			if (await this.options.deviceKeyStore.available()) return await this.options.deviceKeyStore.protect(key)
		} catch {
			// Keep password unlock available when the OS key store is unavailable.
		}
		return undefined
	}
	/** Return an OS-protected key for solver startup without opening the UI/API gate. */
	async backgroundKey(): Promise<Buffer | undefined> {
		const record = this.record()
		if (!record?.deviceKey || !(await this.options.deviceKeyStore.available())) return undefined
		const key = await this.options.deviceKeyStore.unprotect(record.deviceKey)
		try {
			this.checkConfig(key)
			return key
		} catch (error) {
			key.fill(0)
			throw error
		}
	}
	private async wrapPassword(key: Buffer, password: unknown, confirmation: unknown): Promise<Credentials> {
		if (typeof password !== "string" || password.length < 12 || password.length > 1024)
			throw new Error("Use a password with 12–1024 characters")
		if (password !== confirmation) throw new Error("Passwords do not match")
		const salt = randomBytes(16).toString("hex")
		const wrappingKey = await passwordKey(password, salt)
		try {
			return { salt, wrappedKey: sealSecret(key, wrappingKey, "simplex/password/v1") }
		} finally {
			wrappingKey.fill(0)
		}
	}
	private stage(key: Buffer, credentials: Credentials, revision: string | undefined, restartAllowed: boolean): void {
		const { code, wrappedKey } = createRecoveryKey(key)
		this.pending = {
			key,
			record: { ...credentials, version: 2, recoveryKey: wrappedKey },
			revision,
			code,
			restartAllowed,
			committed: false,
			expiresAt: Date.now() + RECOVERY_SESSION_MS,
		}
		this.scheduleExpiry()
	}
	private async activate(key: Buffer): Promise<void> {
		await this.options.start(key)
		this.key = key
		this.options.onUnlocked?.()
	}
	async unlock(request: UnlockRequest): Promise<void> {
		await this.exclusive(async () => {
			if (this.key) return
			if (this.pending || this.recovery) throw new Error("Finish or cancel the current recovery step first")
			const revision = this.revision()
			const record = this.record()
			let key: Buffer | undefined
			try {
				if (!record) {
					if (request.method !== "create") throw new Error("Create a password to continue")
					key = randomBytes(32)
					const credentials = await this.wrapPassword(key, request.password, request.confirmation)
					credentials.deviceKey = await this.wrapDeviceKey(key)
					this.checkConfig(key)
					if (request.useBiometrics === true) credentials.biometricKey = await this.enrollBiometrics(key)
					// A new profile commits only after its recovery code is acknowledged.
					this.stage(key, credentials, revision, request.restartSolver === true)
					key = undefined
					return
				}
				if (request.method === "biometric") key = await this.biometricKey(record)
				else {
					if (
						request.method !== "password" ||
						typeof request.password !== "string" ||
						!request.password.length ||
						request.password.length > 1024
					)
						throw new Error("Enter your password")
					const wrappingKey = await passwordKey(request.password, record.salt)
					try {
						key = openSecret(record.wrappedKey, wrappingKey, "simplex/password/v1")
					} catch {
						throw new Error("Incorrect password or damaged security file")
					} finally {
						wrappingKey.fill(0)
					}
				}
				this.checkConfig(key)
				record.deviceKey = (await this.wrapDeviceKey(key)) ?? record.deviceKey
				if (request.useBiometrics === true && request.method !== "biometric")
					record.biometricKey = await this.enrollBiometrics(key)
				await this.persistAndPrepare(record, key, revision, request.restartSolver === true)
				await this.activate(key)
				key = undefined
			} finally {
				key?.fill(0)
			}
		}, true)
	}
	/** Verify recovery without unlocking APIs, changing the password, or touching the solver. */
	async recover(request: UnlockRequest): Promise<void> {
		await this.exclusive(async () => {
			if (this.key || this.pending || this.recovery) throw new Error("Finish or cancel the current step first")
			const revision = this.revision()
			const record = this.record()
			if (!record) throw new Error("No saved configuration to recover")
			let key: Buffer | undefined
			try {
				if (request.method === "biometric") key = await this.biometricKey(record)
				else if (request.method === "code") key = recoverConfigKey(request.recoveryCode, record.recoveryKey)
				else throw new Error("This recovery method is not available")
				this.checkConfig(key)
				this.recovery = { key, record, revision, expiresAt: Date.now() + RECOVERY_SESSION_MS }
				this.scheduleExpiry()
				key = undefined
			} finally {
				key?.fill(0)
			}
		}, true)
	}
	async resetPassword(request: UnlockRequest): Promise<void> {
		await this.exclusive(async () => {
			const recovery = this.recovery
			if (!recovery) throw new Error("Verify your recovery code or Touch ID again")
			const password = await this.wrapPassword(recovery.key, request.password, request.confirmation)
			const deviceKey = (await this.wrapDeviceKey(recovery.key)) ?? recovery.record.deviceKey
			this.stage(
				recovery.key,
				{ ...recovery.record, ...password, deviceKey },
				recovery.revision,
				request.restartSolver === true,
			)
			this.recovery = undefined
		})
	}
	/** Plaintext codes exist only in this short-lived authenticated session, never on disk. */
	recoveryCode(): string {
		this.expire()
		if (!this.pending) throw new Error("Sign in again to set up recovery")
		return this.pending.code
	}
	async cancelRecovery(): Promise<void> {
		await this.exclusive(async () => {
			this.recovery?.key.fill(0)
			this.recovery = undefined
			this.pending?.key.fill(0)
			this.pending = undefined
		})
	}
	private async persistAndPrepare(
		record: VaultRecord,
		key: Buffer,
		revision: string | undefined,
		restartAllowed: boolean,
	): Promise<void> {
		if (this.revision() !== revision) throw new Error("Security settings changed. Cancel and sign in again.")
		this.checkConfig(key)
		await this.options.prepare(restartAllowed)
		if (this.revision() !== revision) throw new Error("Security settings changed. Cancel and sign in again.")
		mkdirSync(this.options.dataDir, { recursive: true, mode: 0o700 })
		writeSecretAtomic(this.vaultPath, JSON.stringify(record) + "\n")
		if (existsSync(this.configPath)) {
			const content = readFileSync(this.configPath, "utf8")
			if (isEncryptedConfig(content)) decryptConfig(content, key)
			else writeSecretAtomic(this.configPath, encryptConfig(content, key))
		}
	}
	async confirmRecovery(saved: unknown): Promise<void> {
		await this.exclusive(async () => {
			const pending = this.pending
			if (!pending || saved !== true) throw new Error("Save your recovery code before continuing")
			if (!pending.committed) {
				try {
					await this.persistAndPrepare(pending.record, pending.key, pending.revision, pending.restartAllowed)
				} finally {
					// Metadata may have committed before a later migration error. Preserve retryability.
					if (this.revision() === JSON.stringify(pending.record) + "\n") pending.revision = this.revision()
				}
				pending.committed = true
			}
			if (this.revision() !== pending.revision)
				throw new Error("Security settings changed. Cancel and sign in again.")
			await this.activate(pending.key)
			this.pending = undefined
		})
	}
	async handle(request: Request): Promise<Response> {
		const json = (body: unknown, status = 200) =>
			new Response(JSON.stringify(body), {
				status,
				headers: { "Content-Type": "application/json", "Cache-Control": "no-store" },
			})
		const path = new URL(request.url).pathname
		const origin = request.headers.get("origin")
		const fromUi = request.headers.get("x-simplex-ui") === "1" && (!origin || origin === "simplex://local")
		try {
			if (path === "/api/desktop/security" && request.method === "GET") return json(await this.state())
			if (path === "/api/desktop/recovery-code" && request.method === "GET") {
				if (!fromUi) return json({ error: "Forbidden" }, 403)
				return json({ code: this.recoveryCode() })
			}
			const routes = ["unlock", "recover", "reset-password", "confirm-recovery", "cancel-recovery"]
			const route = path.replace("/api/desktop/", "")
			if (!routes.includes(route)) return json({ error: "Not found" }, 404)
			if (request.method !== "POST") return json({ error: "Method not allowed" }, 405)
			if (!fromUi) return json({ error: "Forbidden" }, 403)
			const reader = request.body?.getReader()
			if (!reader) return json({ error: "Missing request" }, 400)
			let text = "",
				size = 0
			const decoder = new TextDecoder()
			try {
				for (;;) {
					const { value, done } = await reader.read()
					if (done) break
					size += value.length
					if (size > 16_384) return json({ error: "Request too large" }, 413)
					text += decoder.decode(value, { stream: true })
				}
				text += decoder.decode()
			} finally {
				await reader.cancel()
			}
			let body: UnlockRequest & { saved?: unknown }
			try {
				body = JSON.parse(text)
			} catch {
				return json({ error: "Invalid request" }, 400)
			}
			if (!body || typeof body !== "object" || Array.isArray(body)) return json({ error: "Invalid request" }, 400)
			switch (route) {
				case "unlock":
					await this.unlock(body)
					break
				case "recover":
					await this.recover(body)
					break
				case "reset-password":
					await this.resetPassword(body)
					break
				case "confirm-recovery":
					await this.confirmRecovery(body.saved)
					break
				case "cancel-recovery":
					await this.cancelRecovery()
					break
			}
			return json({ ok: true })
		} catch (error) {
			return json({ error: error instanceof Error ? error.message : "Could not unlock Simplex" }, 400)
		}
	}
}
