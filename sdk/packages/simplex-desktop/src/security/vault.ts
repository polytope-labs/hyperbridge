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
import { isPasskeyCredential, type PasskeyCredential, type PasskeyUnlock } from "./passkeys"

type VaultRecord = {
	version: 2 | 3 | 4
	profileId?: string
	salt?: string
	wrappedKey?: SealedSecret
	passkey?: PasskeyCredential & { key: string }
	recoveryKey: SealedSecret
	deviceKey?: string
}
/** Credential wrappers before a recovery code is attached; never persisted in this shape. */
type Credentials = Omit<VaultRecord, "version" | "recoveryKey">
export interface DesktopAccessState {
	mode: "create" | "unlock" | "reset-password" | "save-recovery" | "unlocked"
	passkeyAvailable: boolean
	passkeyEnabled: boolean
	passkeyMechanism?: string
	passkeyUnavailableReason?: string
	passkeyMigrationRequired?: boolean
	passkeyEnrollmentAvailable?: boolean
	passwordEnabled: boolean
	recoveryEnabled: boolean
	backgroundResumeAvailable: boolean
	secureDeviceStorageAvailable: boolean
	needsRestart: boolean
}
type UnlockRequest = {
	method?: unknown
	password?: unknown
	confirmation?: unknown
	usePasskey?: unknown
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
	private generation = 0
	private passkeyAttempt = 0
	private passkeyActive = false
	private passkeyFinalState?: "committing" | "succeeded" | "failed" | "cancelled"
	private activating = false
	private busy = false
	private nextAttempt = 0
	constructor(
		private readonly options: {
			dataDir: string
			passkeys?: PasskeyUnlock
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
		this.generation++
		this.cancelPasskey()
		clearTimeout(this.expiryTimer)
		this.key?.fill(0)
		this.recovery?.key.fill(0)
		this.pending?.key.fill(0)
		this.key = undefined
		this.recovery = undefined
		this.pending = undefined
		this.passkeyFinalState = undefined
		this.options.passkeys?.resetStatus?.()
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
			// Profiles from before passkeys may carry a Touch ID wrapper; it is ignored and dropped on the next save.
			const { biometricKey: _legacyTouchId, ...parsed } = JSON.parse(content) as VaultRecord & {
				biometricKey?: unknown
			}
			record = parsed
		} catch {
			throw new Error("Unsupported or invalid desktop security file")
		}
		if (
			!record ||
			(record.version !== 2 && record.version !== 3 && record.version !== 4) ||
			(record.profileId !== undefined &&
				(typeof record.profileId !== "string" || !/^[A-Za-z0-9_-]{22}$/.test(record.profileId))) ||
			(record.version === 4 && !record.profileId) ||
			(record.passkey?.mechanism === "native-macos" && (record.version !== 4 || !record.profileId)) ||
			(record.version === 2 && (!record.salt || !record.wrappedKey)) ||
			(record.salt !== undefined && (typeof record.salt !== "string" || !/^[a-f0-9]{32}$/.test(record.salt))) ||
			Boolean(record.salt) !== Boolean(record.wrappedKey) ||
			(!record.wrappedKey && !record.passkey) ||
			(record.passkey !== undefined &&
				(!isPasskeyCredential(record.passkey) ||
					typeof record.passkey.key !== "string" ||
					!/^(?:[a-f0-9]{2})+$/.test(record.passkey.key))) ||
			!record.recoveryKey ||
			(record.deviceKey !== undefined &&
				(typeof record.deviceKey !== "string" || !/^(?:[a-f0-9]{2})+$/.test(record.deviceKey)))
		)
			throw new Error("Unsupported or invalid desktop security file")
		return record
	}
	async state(): Promise<DesktopAccessState> {
		this.expire()
		const record = this.record()
		const secureDeviceStorageAvailable = await this.options.deviceKeyStore.available()
		const capability = this.options.passkeys?.capability?.()
		const migration = Boolean(
			record?.passkey && this.options.passkeys?.supports && !this.options.passkeys.supports(record.passkey),
		)
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
			passkeyAvailable:
				Boolean(this.options.passkeys?.available()) &&
				secureDeviceStorageAvailable &&
				(!migration || this.recovery !== undefined),
			passkeyEnabled: Boolean(record?.passkey),
			passkeyMechanism: this.options.passkeys?.mechanism,
			passkeyUnavailableReason:
				migration && !this.recovery
					? record?.wrappedKey
						? "Create a native Mac passkey after signing in with your password or recovery code."
						: "Use your recovery code to create a native Mac passkey."
					: capability?.reason,
			passkeyMigrationRequired: migration,
			passkeyEnrollmentAvailable: Boolean(this.options.passkeys?.available()) && secureDeviceStorageAvailable,
			passwordEnabled: Boolean(record?.wrappedKey),
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
		this.passkeyFinalState = undefined
		this.options.passkeys?.resetStatus?.()
		try {
			await action()
		} catch (error) {
			if (this.passkeyFinalState === "committing") this.passkeyFinalState = "failed"
			throw error
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
	private async requirePasskeys(): Promise<PasskeyUnlock> {
		if (!this.options.passkeys?.available() || !(await this.options.deviceKeyStore.available()))
			throw new Error(
				this.options.passkeys?.capability?.().reason ??
					"Passkeys require macOS or Windows with secure OS key storage. Use another sign-in method.",
			)
		return this.options.passkeys
	}
	/** True only during a passkey operation, not password verification or solver activation. */
	isPasskeyPending(): boolean {
		return this.passkeyActive
	}
	cancelPasskey(): void {
		// Solver activation is the commit boundary: finishing it keeps solver and UI access consistent.
		if (this.activating) return
		if (this.passkeyFinalState === "committing") this.passkeyFinalState = "cancelled"
		this.passkeyAttempt++
		this.options.passkeys?.cancel()
	}
	/** Teardown cancels passkey work without discarding password or authorized recovery state. */
	cancelPendingAccess(): void {
		this.cancelPasskey()
	}
	/** A cancel before a ceremony starts must not be lost. */
	private async ceremony<T>(attempt: number, run: () => Promise<T>): Promise<T> {
		if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
		this.passkeyActive = true
		this.passkeyFinalState = undefined
		try {
			const result = await run()
			if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
			return result
		} finally {
			this.passkeyActive = false
		}
	}
	/** Native replacement uses a fresh handle, preserving the old credential until commit. */
	private async enrollPasskey(
		key: Buffer,
		userId: string | undefined,
		profileId: string,
		attempt = this.passkeyAttempt,
	): Promise<NonNullable<VaultRecord["passkey"]>> {
		const generation = this.generation
		if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
		const passkeys = await this.requirePasskeys()
		const wrappedKey = await this.options.deviceKeyStore.protect(key)
		this.checkGeneration(generation)
		let credential = await this.ceremony(attempt, () =>
			passkeys.mechanism === "native-macos"
				? passkeys.register(undefined, { profileId })
				: passkeys.register(userId),
		)
		if (credential.mechanism === "native-macos")
			credential = await this.ceremony(attempt, () => passkeys.authenticate(credential))
		this.checkGeneration(generation)
		if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
		return { ...credential, key: wrappedKey }
	}
	private async passkeyKey(record: VaultRecord): Promise<Buffer> {
		if (!record.passkey) throw new Error("No passkey is enrolled for this profile")
		if (this.options.passkeys?.supports && !this.options.passkeys.supports(record.passkey))
			throw new Error(
				record.wrappedKey
					? "This passkey needs migration. Use your password or recovery code to create a native Mac passkey."
					: "This passkey needs migration. Use your recovery code to create a native Mac passkey.",
			)
		const generation = this.generation
		const attempt = this.passkeyAttempt
		const passkeys = await this.requirePasskeys()
		this.checkGeneration(generation)
		const { key: wrappedKey, ...savedCredential } = record.passkey
		const credential = await this.ceremony(attempt, () => passkeys.authenticate(savedCredential))
		this.checkGeneration(generation)
		if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
		this.passkeyFinalState = "committing"
		// Only a verified assertion can release the OS-protected key to the dashboard.
		const key = await this.options.deviceKeyStore.unprotect(wrappedKey)
		try {
			this.checkGeneration(generation)
			if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
			record.passkey = { ...credential, key: wrappedKey }
			return key
		} catch (error) {
			key.fill(0)
			throw error
		}
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
			record: {
				...credentials,
				version: credentials.passkey?.mechanism === "native-macos" ? 4 : credentials.passkey ? 3 : 2,
				profileId:
					credentials.profileId ??
					(credentials.passkey?.mechanism === "native-macos"
						? randomBytes(16).toString("base64url")
						: undefined),
				recoveryKey: wrappedKey,
			},
			revision,
			code,
			restartAllowed,
			committed: false,
			expiresAt: Date.now() + RECOVERY_SESSION_MS,
		}
		this.scheduleExpiry()
	}
	private async activate(key: Buffer, passkeyAttempt?: number): Promise<void> {
		const generation = this.generation
		if (passkeyAttempt !== undefined && passkeyAttempt !== this.passkeyAttempt)
			throw new Error("Passkey cancelled. Try again.")
		this.activating = true
		try {
			await this.options.start(key)
			this.checkGeneration(generation)
			this.key = key
			if (passkeyAttempt !== undefined) this.passkeyFinalState = "succeeded"
			this.options.onUnlocked?.()
		} finally {
			this.activating = false
		}
	}
	private checkGeneration(generation: number): void {
		if (generation !== this.generation) throw new Error("Sign-in cancelled. Try again.")
	}
	/** Resolves to a warning when the password unlock succeeded but the optional passkey was not saved. */
	async unlock(request: UnlockRequest): Promise<string | undefined> {
		let warning: string | undefined
		await this.exclusive(async () => {
			if (this.key) return
			if (this.pending || this.recovery) throw new Error("Finish or cancel the current recovery step first")
			const generation = this.generation
			const revision = this.revision()
			const record = this.record()
			let key: Buffer | undefined
			const attempt = this.passkeyAttempt
			const passkeyAttempt =
				request.method === "passkey" || request.method === "create-passkey" ? attempt : undefined
			try {
				if (!record) {
					if (request.method !== "create" && request.method !== "create-passkey")
						throw new Error("Create a passkey or password to continue")
					key = randomBytes(32)
					const profileId = randomBytes(16).toString("base64url")
					const credentials: Credentials =
						request.method === "create-passkey"
							? {
									passkey: await this.enrollPasskey(key, undefined, profileId, attempt),
									profileId:
										this.options.passkeys?.mechanism === "native-macos" ? profileId : undefined,
								}
							: await this.wrapPassword(key, request.password, request.confirmation)
					credentials.deviceKey = await this.wrapDeviceKey(key)
					this.checkConfig(key)
					this.checkGeneration(generation)
					if (passkeyAttempt !== undefined && passkeyAttempt !== this.passkeyAttempt)
						throw new Error("Passkey cancelled. Try again.")
					// A new profile commits only after its recovery code is acknowledged.
					this.stage(key, credentials, revision, request.restartSolver === true)
					key = undefined
					return
				}
				if (request.method === "passkey") key = await this.passkeyKey(record)
				else {
					if (
						request.method !== "password" ||
						typeof request.password !== "string" ||
						!request.password.length ||
						request.password.length > 1024
					)
						throw new Error("Enter your password")
					if (!record.salt || !record.wrappedKey) throw new Error("Use your passkey or recovery code")
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
				if (request.usePasskey === true && request.method !== "passkey") {
					// The password already proved access; a declined passkey must not block this unlock.
					try {
						const profileId = record.profileId ?? randomBytes(16).toString("base64url")
						record.passkey = await this.enrollPasskey(key, record.passkey?.userId, profileId, attempt)
						record.version = record.passkey.mechanism === "native-macos" ? 4 : 3
						if (record.version === 4) record.profileId = profileId
					} catch {
						this.checkGeneration(generation)
						warning = "Passkey not saved. You can create one the next time you sign in with your password."
					}
				}
				record.deviceKey = (await this.wrapDeviceKey(key)) ?? record.deviceKey
				this.checkGeneration(generation)
				await this.persistAndPrepare(record, key, revision, request.restartSolver === true, passkeyAttempt)
				this.checkGeneration(generation)
				await this.activate(key, passkeyAttempt)
				key = undefined
			} finally {
				key?.fill(0)
			}
		}, true)
		return warning
	}
	/** Verify recovery without unlocking APIs, changing the password, or touching the solver. */
	async recover(request: UnlockRequest): Promise<void> {
		await this.exclusive(async () => {
			if (this.key || this.pending || this.recovery) throw new Error("Finish or cancel the current step first")
			const generation = this.generation
			const revision = this.revision()
			const record = this.record()
			if (!record) throw new Error("No saved configuration to recover")
			const passkeyAttempt = request.method === "passkey" ? this.passkeyAttempt : undefined
			let key: Buffer | undefined
			try {
				if (request.method === "passkey") key = await this.passkeyKey(record)
				else if (request.method === "code") key = recoverConfigKey(request.recoveryCode, record.recoveryKey)
				else throw new Error("This recovery method is not available")
				this.checkConfig(key)
				this.checkGeneration(generation)
				if (passkeyAttempt !== undefined && passkeyAttempt !== this.passkeyAttempt)
					throw new Error("Passkey cancelled. Try again.")
				this.recovery = { key, record, revision, expiresAt: Date.now() + RECOVERY_SESSION_MS }
				if (request.method === "passkey") this.passkeyFinalState = "succeeded"
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
			if (!recovery) throw new Error("Verify your recovery code or sign-in method again")
			const password = await this.wrapPassword(recovery.key, request.password, request.confirmation)
			const deviceKey = (await this.wrapDeviceKey(recovery.key)) ?? recovery.record.deviceKey
			if (this.recovery !== recovery) throw new Error("Recovery cancelled. Verify your recovery code again.")
			this.stage(
				recovery.key,
				{ ...recovery.record, ...password, passkey: undefined, deviceKey },
				recovery.revision,
				request.restartSolver === true,
			)
			this.recovery = undefined
		})
	}
	async resetPasskey(request: UnlockRequest): Promise<void> {
		await this.exclusive(async () => {
			const recovery = this.recovery
			if (!recovery) throw new Error("Verify your recovery code or sign-in method again")
			const attempt = this.passkeyAttempt
			const profileId = recovery.record.profileId ?? randomBytes(16).toString("base64url")
			const passkey = await this.enrollPasskey(recovery.key, recovery.record.passkey?.userId, profileId, attempt)
			const deviceKey = (await this.wrapDeviceKey(recovery.key)) ?? recovery.record.deviceKey
			if (attempt !== this.passkeyAttempt) throw new Error("Passkey cancelled. Try again.")
			if (this.recovery !== recovery) throw new Error("Recovery cancelled. Verify your recovery code again.")
			this.stage(
				recovery.key,
				{
					...recovery.record,
					passkey,
					deviceKey,
					profileId: passkey.mechanism === "native-macos" ? profileId : recovery.record.profileId,
				},
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
		passkeyAttempt?: number,
	): Promise<void> {
		const generation = this.generation
		if (this.revision() !== revision) throw new Error("Security settings changed. Cancel and sign in again.")
		this.checkConfig(key)
		await this.options.prepare(restartAllowed)
		this.checkGeneration(generation)
		if (passkeyAttempt !== undefined && passkeyAttempt !== this.passkeyAttempt)
			throw new Error("Passkey cancelled. Try again.")
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
			if (path === "/api/desktop/passkey-status" && request.method === "GET") {
				const native = this.options.passkeys?.status?.()
				return json({
					pending: this.isPasskeyPending(),
					...native,
					...(native && this.passkeyFinalState ? { state: this.passkeyFinalState } : {}),
				})
			}
			if (path === "/api/desktop/recovery-code" && request.method === "GET") {
				if (!fromUi) return json({ error: "Forbidden" }, 403)
				return json({ code: this.recoveryCode() })
			}
			const routes = [
				"unlock",
				"recover",
				"reset-password",
				"reset-passkey",
				"confirm-recovery",
				"cancel-recovery",
				"cancel-passkey",
			]
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
			let warning: string | undefined
			switch (route) {
				case "unlock":
					warning = await this.unlock(body)
					break
				case "recover":
					await this.recover(body)
					break
				case "reset-password":
					await this.resetPassword(body)
					break
				case "reset-passkey":
					await this.resetPasskey(body)
					break
				case "cancel-passkey":
					this.cancelPasskey()
					break
				case "confirm-recovery":
					await this.confirmRecovery(body.saved)
					break
				case "cancel-recovery":
					await this.cancelRecovery()
					break
			}
			return json(warning ? { ok: true, warning } : { ok: true })
		} catch (error) {
			return json(
				{
					error: error instanceof Error ? error.message : "Could not unlock Simplex",
					code:
						error instanceof Error && "code" in error && typeof error.code === "string"
							? error.code
							: undefined,
				},
				400,
			)
		}
	}
}
