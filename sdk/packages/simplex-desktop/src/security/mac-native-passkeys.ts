import { randomBytes } from "node:crypto"
import { createRequire } from "node:module"
import { existsSync, readFileSync } from "node:fs"
import { join } from "node:path"
import {
	generateRegistrationOptions,
	generateAuthenticationOptions,
	verifyRegistrationResponse,
	verifyAuthenticationResponse,
	type RegistrationResponseJSON,
	type AuthenticationResponseJSON,
} from "@simplewebauthn/server"
import { isPasskeyRpId, type PasskeyCredential, type PasskeyUnlock } from "./passkeys"
import passkeyConfiguration from "../../resources/passkeys.json"

export interface MacPasskeyBridge {
	capabilities(rpId: string): { available: boolean; reason?: string }
	begin(request: string, windowHandle: Buffer): Promise<string>
	cancel(operationId: string): void
}
export type NativePasskeyState =
	| "idle"
	| "preparing"
	| "awaiting-user"
	| "verifying"
	| "succeeded"
	| "cancelled"
	| "failed"
const ERROR_CODES = [
	"CANCELLED",
	"TIMEOUT",
	"NOT_SUPPORTED",
	"APP_NOT_PROVISIONED",
	"NATIVE_FAILED",
	"NATIVE_BUSY",
	"INVALID_RESPONSE",
	"VERIFICATION_FAILED",
	"MIGRATION_REQUIRED",
]
function passkeyError(code: string, message: string): Error & { code: string } {
	return Object.assign(new Error(message), { code })
}

/** A host-only adapter: native responses still pass the full WebAuthn verifier. */
export class MacNativePasskeys implements PasskeyUnlock {
	readonly mechanism = "native-macos" as const
	private active?: { id: string; reject(error: Error): void }
	private generation = 0
	private state: NativePasskeyState = "idle"
	private operationId?: string
	private errorCode?: string
	constructor(
		private readonly options: {
			rpId?: string
			bridge?: MacPasskeyBridge
			unavailableReason?: string
			windowHandle(): Buffer
			timeoutMs?: number
		},
	) {}
	available(): boolean {
		return this.capability().available
	}
	capability(): { available: boolean; reason?: string } {
		if (!this.options.rpId)
			return {
				available: false,
				reason: "Native passkeys are not configured in this build. Use a password or recovery code.",
			}
		if (!isPasskeyRpId(this.options.rpId))
			return {
				available: false,
				reason: "The native passkey domain is invalid. Use a password or recovery code.",
			}
		if (!this.options.bridge)
			return {
				available: false,
				reason:
					this.options.unavailableReason ??
					"The native passkey module is unavailable. Use a password or recovery code.",
			}
		try {
			const capability = this.options.bridge.capabilities(this.options.rpId)
			return capability.available
				? capability
				: {
						available: false,
						reason:
							capability.reason ??
							"This installation cannot use Mac passkeys. Use a password or recovery code.",
					}
		} catch {
			return { available: false, reason: "Native passkeys are unavailable. Use a password or recovery code." }
		}
	}
	supports(credential: PasskeyCredential): boolean {
		return credential.mechanism === this.mechanism && credential.rpId === this.options.rpId
	}
	status() {
		return {
			mechanism: this.mechanism,
			state: this.state,
			operationId: this.operationId,
			errorCode: this.errorCode,
		}
	}
	resetStatus(): void {
		if (this.active) return
		this.state = "idle"
		this.operationId = undefined
		this.errorCode = undefined
	}
	cancel(): void {
		this.abort(passkeyError("CANCELLED", "Passkey cancelled. Try again."))
	}
	private abort(error: Error): void {
		this.generation++
		const active = this.active
		if (!active) return
		this.errorCode = "code" in error ? String(error.code) : "CANCELLED"
		this.state = this.errorCode === "CANCELLED" ? "cancelled" : "failed"
		active.reject(error)
		try {
			this.options.bridge?.cancel(active.id)
		} catch {
			/* The generation also rejects late native results. */
		}
	}
	async register(_userId?: string, context?: { profileId: string }): Promise<PasskeyCredential> {
		// A new handle preserves the previously enrolled credential until vault commit.
		const userId = randomBytes(16).toString("base64url")
		return this.ceremony(async (generation, operationId) => {
			const options = await generateRegistrationOptions({
				rpName: "Simplex Desktop",
				rpID: this.options.rpId!,
				userID: new Uint8Array(Buffer.from(userId, "base64url")),
				userName: `Simplex profile ${context?.profileId.slice(0, 8) ?? "local"} (${new Date().toISOString()})`,
				userDisplayName: "Simplex Desktop profile",
				attestationType: "none",
				supportedAlgorithmIDs: [-7],
				authenticatorSelection: { residentKey: "required", userVerification: "required" },
			})
			const response = await this.native(generation, {
				kind: "register",
				operationId,
				rpId: this.options.rpId,
				challenge: options.challenge,
				userId,
				userName: options.user.name,
			})
			this.state = "verifying"
			const result = await verifyRegistrationResponse({
				response: response as RegistrationResponseJSON,
				expectedChallenge: options.challenge,
				expectedOrigin: `https://${this.options.rpId}`,
				expectedRPID: this.options.rpId!,
				requireUserVerification: true,
				supportedAlgorithmIDs: [-7],
			})
			if (!result.verified) throw new Error("Passkey registration could not be verified.")
			const credential = result.registrationInfo.credential
			return {
				id: credential.id,
				publicKey: Buffer.from(credential.publicKey).toString("base64url"),
				counter: credential.counter,
				userId,
				rpId: this.options.rpId,
				mechanism: this.mechanism,
			}
		})
	}
	async authenticate(credential: PasskeyCredential): Promise<PasskeyCredential> {
		if (!this.supports(credential))
			throw passkeyError(
				"MIGRATION_REQUIRED",
				"This passkey needs migration. Sign in with your password or recovery code to create a native Mac passkey.",
			)
		return this.ceremony(async (generation, operationId) => {
			const options = await generateAuthenticationOptions({
				rpID: credential.rpId!,
				userVerification: "required",
				allowCredentials: [{ id: credential.id }],
			})
			const response = (await this.native(generation, {
				kind: "authenticate",
				operationId,
				rpId: credential.rpId,
				challenge: options.challenge,
				credentialId: credential.id,
			})) as AuthenticationResponseJSON
			if (
				response.id !== credential.id ||
				(response.response.userHandle && response.response.userHandle !== credential.userId)
			)
				throw new Error("The passkey does not match this profile.")
			this.state = "verifying"
			const result = await verifyAuthenticationResponse({
				response,
				credential: {
					id: credential.id,
					publicKey: new Uint8Array(Buffer.from(credential.publicKey, "base64url")),
					counter: credential.counter,
				},
				expectedChallenge: options.challenge,
				expectedOrigin: `https://${credential.rpId}`,
				expectedRPID: credential.rpId!,
				requireUserVerification: true,
			})
			if (!result.verified) throw new Error("Passkey authentication could not be verified.")
			return { ...credential, counter: result.authenticationInfo.newCounter }
		})
	}
	private async native(generation: number, request: object): Promise<unknown> {
		if (generation !== this.generation) throw new Error("Passkey cancelled. Try again.")
		const handle = this.options.windowHandle()
		this.state = "awaiting-user"
		const json = await this.options.bridge!.begin(JSON.stringify(request), handle)
		if (generation !== this.generation) throw new Error("Passkey cancelled. Try again.")
		if (typeof json !== "string" || Buffer.byteLength(json) > 65_536)
			throw new Error("Invalid native passkey response.")
		return JSON.parse(json)
	}
	private async ceremony<T>(run: (generation: number, operationId: string) => Promise<T>): Promise<T> {
		if (this.active) throw new Error("A passkey request is already in progress.")
		const capability = this.capability()
		if (!capability.available) throw new Error(capability.reason ?? "Native passkeys are unavailable.")
		const generation = this.generation
		const id = randomBytes(16).toString("hex")
		this.operationId = id
		this.state = "preparing"
		this.errorCode = undefined
		let reject!: (error: Error) => void
		const cancelled = new Promise<never>((_resolve, no) => {
			reject = no
		})
		this.active = { id, reject }
		const timer = setTimeout(() => {
			this.abort(passkeyError("TIMEOUT", "Passkey request expired. Try again."))
		}, this.options.timeoutMs ?? 120_000)
		try {
			const result = await Promise.race([run(generation, id), cancelled])
			if (generation !== this.generation) throw new Error("Passkey cancelled. Try again.")
			this.state = "succeeded"
			return result
		} catch (error) {
			if (error instanceof Error && "code" in error && ERROR_CODES.includes(String(error.code))) {
				this.errorCode = String(error.code)
				this.state = this.errorCode === "CANCELLED" ? "cancelled" : "failed"
				throw error
			}
			this.errorCode = "VERIFICATION_FAILED"
			this.state = "failed"
			throw passkeyError(
				this.errorCode,
				"Passkey verification failed. Retry or use your password or recovery code.",
			)
		} finally {
			clearTimeout(timer)
			if (this.active?.id === id) this.active = undefined
		}
	}
}

export function loadMacNativePasskeys(options: {
	isPackaged: boolean
	resourcesPath: string
	packageRoot: string
	windowHandle(): Buffer
}): MacNativePasskeys {
	try {
		const metadata = JSON.parse(readFileSync(join(options.packageRoot, "package.json"), "utf8"))
		const rpId = options.isPackaged
			? metadata.simplexPasskeyRpId
			: process.env.SIMPLEX_PASSKEY_RP_ID || passkeyConfiguration.rpId
		const path = options.isPackaged
			? join(options.resourcesPath, "desktop", "native", "mac-passkeys.node")
			: join(options.packageRoot, "build", "native", `darwin-${process.arch}`, "mac-passkeys.node")
		const bridge = existsSync(path) ? (createRequire(import.meta.url)(path) as MacPasskeyBridge) : undefined
		return new MacNativePasskeys({ rpId, bridge, windowHandle: options.windowHandle })
	} catch {
		return new MacNativePasskeys({
			windowHandle: options.windowHandle,
			unavailableReason: "The native passkey module could not load. Use a password or recovery code.",
		})
	}
}
