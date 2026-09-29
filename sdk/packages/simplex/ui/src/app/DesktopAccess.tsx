import { useCallback, useEffect, useState, type ReactNode } from "react"
import { api } from "../api"
import { isNativeDesktopProtocol } from "../lib/runtime"

type AccessState = {
	mode: "create" | "unlock" | "reset-password" | "save-recovery" | "unlocked"
	biometricAvailable: boolean
	biometricEnabled: boolean
	recoveryEnabled: boolean
	backgroundResumeAvailable: boolean
	secureDeviceStorageAvailable: boolean
	needsRestart: boolean
}
type Perform = (route: string, body?: Record<string, unknown>) => Promise<void>

/** The host enforces these states too; changing renderer state cannot unlock the API. */
export function DesktopAccess({ children }: { children: ReactNode }) {
	return isNativeDesktopProtocol(window.location.protocol) ? <NativeAccess>{children}</NativeAccess> : children
}

function NativeAccess({ children }: { children: ReactNode }) {
	const [access, setAccess] = useState<AccessState>()
	const [code, setCode] = useState("")
	const [error, setError] = useState<string>()
	const [forgot, setForgot] = useState(false)
	const [pending, setPending] = useState(false)
	const refresh = useCallback(async () => {
		const next = await api.get<AccessState>("/api/desktop/security")
		const recovery =
			next.mode === "save-recovery" ? await api.get<{ code: string }>("/api/desktop/recovery-code") : undefined
		setCode(recovery?.code ?? "")
		setAccess(next)
	}, [])
	useEffect(() => {
		void refresh().catch((cause) => setError(message(cause)))
	}, [refresh])
	const perform: Perform = async (route, body) => {
		setPending(true)
		setError(undefined)
		try {
			await api.post(`/api/desktop/${route}`, body)
			setForgot(false)
			await refresh()
		} catch (cause) {
			await refresh().catch(() => {})
			setError(message(cause))
		} finally {
			setPending(false)
		}
	}
	if (access?.mode === "unlocked") return children
	const cancel = () => {
		void perform("cancel-recovery")
	}
	return (
		<main className="desktop-unlock">
			<section className="card" aria-label="Simplex Desktop access">
				<img src="./icons/mobile-logo.svg" alt="" width="48" height="48" />
				<span className="eyebrow">Simplex Desktop</span>
				{error && (
					<p className="desktop-unlock-error" role="alert">
						{error}
					</p>
				)}
				{!access ? (
					<button type="button" onClick={() => void refresh().catch((cause) => setError(message(cause)))}>
						Try again
					</button>
				) : (
					<fieldset disabled={pending}>
						{access.mode === "save-recovery" ? (
							<SaveRecovery code={code} perform={perform} cancel={cancel} />
						) : access.mode === "reset-password" ? (
							<NewPassword access={access} perform={perform} cancel={cancel} />
						) : forgot && access.mode === "unlock" ? (
							<Recover
								access={access}
								perform={perform}
								cancel={() => {
									setForgot(false)
									setError(undefined)
								}}
							/>
						) : (
							<Login
								access={access}
								perform={perform}
								forgot={() => {
									setForgot(true)
									setError(undefined)
								}}
							/>
						)}
						{pending && (
							<p className="hint" role="status">
								Please wait…
							</p>
						)}
					</fieldset>
				)}
			</section>
		</main>
	)
}

function message(cause: unknown): string {
	return cause instanceof Error ? cause.message : "Could not open Simplex"
}

function PasswordFields({
	creating,
	password,
	confirmation,
	setPassword,
	setConfirmation,
}: {
	creating: boolean
	password: string
	confirmation: string
	setPassword(value: string): void
	setConfirmation(value: string): void
}) {
	return (
		<>
			<label htmlFor="desktop-password">{creating ? "New password" : "Password"}</label>
			<input
				id="desktop-password"
				type="password"
				required
				minLength={creating ? 12 : 1}
				maxLength={1024}
				autoComplete={creating ? "new-password" : "current-password"}
				value={password}
				onChange={(event) => setPassword(event.target.value)}
			/>
			{creating && (
				<>
					<label htmlFor="desktop-confirmation">Confirm password</label>
					<input
						id="desktop-confirmation"
						type="password"
						required
						minLength={12}
						maxLength={1024}
						autoComplete="new-password"
						value={confirmation}
						onChange={(event) => setConfirmation(event.target.value)}
					/>
				</>
			)}
		</>
	)
}

function RestartConsent({
	required,
	checked,
	onChange,
}: {
	required: boolean
	checked: boolean
	onChange(value: boolean): void
}) {
	return required ? (
		<label className="desktop-unlock-choice">
			<input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
			Stop and restart the running solver to protect your settings. In-flight work must finish first.
		</label>
	) : null
}

function Login({ access, perform, forgot }: { access: AccessState; perform: Perform; forgot(): void }) {
	const creating = access.mode === "create"
	const [password, setPassword] = useState("")
	const [confirmation, setConfirmation] = useState("")
	const [useBiometrics, setUseBiometrics] = useState(false)
	const [restartSolver, setRestartSolver] = useState(false)
	const unlock = async (method: string) => {
		await perform("unlock", { method, password, confirmation, useBiometrics, restartSolver })
		setPassword("")
		setConfirmation("")
	}
	return (
		<form
			onSubmit={(event) => {
				event.preventDefault()
				void unlock(creating ? "create" : "password")
			}}
		>
			<h1>{creating ? "Create a password" : "Unlock Simplex"}</h1>
			<p className="hint">
				{creating
					? "Keep your settings and wallet details safe on this device."
					: "Enter your password to continue."}
			</p>
			{!access.secureDeviceStorageAvailable && (
				<p className="hint" role="note">
					Secure key storage is unavailable. After a reboot or update, sign in to resume filling.
				</p>
			)}
			{!creating && access.secureDeviceStorageAvailable && !access.backgroundResumeAvailable && (
				<p className="hint" role="note">
					Sign in once to enable automatic solver restarts on this device.
				</p>
			)}
			<PasswordFields {...{ creating, password, confirmation, setPassword, setConfirmation }} />
			{access.biometricAvailable && !access.biometricEnabled && (
				<label className="desktop-unlock-choice">
					<input
						type="checkbox"
						checked={useBiometrics}
						onChange={(event) => setUseBiometrics(event.target.checked)}
					/>{" "}
					Also enable Touch ID on this Mac
				</label>
			)}
			<RestartConsent required={access.needsRestart} checked={restartSolver} onChange={setRestartSolver} />
			<button type="submit" className="primary" disabled={access.needsRestart && !restartSolver}>
				{creating ? "Continue" : "Unlock"}
			</button>
			{access.biometricAvailable && access.biometricEnabled && (
				<button
					type="button"
					disabled={access.needsRestart && !restartSolver}
					onClick={() => void unlock("biometric")}
				>
					Unlock with Touch ID
				</button>
			)}
			{!creating && (
				<button type="button" onClick={forgot}>
					Forgot password?
				</button>
			)}
		</form>
	)
}

function Recover({ access, perform, cancel }: { access: AccessState; perform: Perform; cancel(): void }) {
	const [recoveryCode, setRecoveryCode] = useState("")
	const biometric = access.biometricAvailable && access.biometricEnabled
	return (
		<form
			onSubmit={(event) => {
				event.preventDefault()
				void perform("recover", { method: "code", recoveryCode })
			}}
		>
			<h1>Reset your password</h1>
			<p className="hint">Verify it's you to choose a new password. Your saved settings will stay intact.</p>
			{access.recoveryEnabled && (
				<>
					<label htmlFor="recovery-code">Recovery code</label>
					<input
						id="recovery-code"
						required
						value={recoveryCode}
						maxLength={256}
						autoComplete="off"
						spellCheck={false}
						onChange={(event) => setRecoveryCode(event.target.value)}
					/>
					<button type="submit" className="primary">
						Continue
					</button>
				</>
			)}
			{biometric && (
				<button type="button" onClick={() => void perform("recover", { method: "biometric" })}>
					Verify with Touch ID
				</button>
			)}
			{!access.recoveryEnabled && !biometric && (
				<p className="hint">
					Recovery wasn't set up for this profile. Sign in with your password to set it up, or restore a
					backup you can unlock. Your existing data will not be deleted.
				</p>
			)}
			{access.recoveryEnabled && (
				<p className="hint">
					Use the code you saved when setting up Simplex. Without a working recovery method, we can't unlock
					your saved settings.
				</p>
			)}
			<button type="button" onClick={cancel}>
				Back to login
			</button>
		</form>
	)
}

function NewPassword({ access, perform, cancel }: { access: AccessState; perform: Perform; cancel(): void }) {
	const [password, setPassword] = useState("")
	const [confirmation, setConfirmation] = useState("")
	const [restartSolver, setRestartSolver] = useState(false)
	return (
		<form
			onSubmit={(event) => {
				event.preventDefault()
				void perform("reset-password", { password, confirmation, restartSolver })
			}}
		>
			<h1>Create a new password</h1>
			<PasswordFields creating {...{ password, confirmation, setPassword, setConfirmation }} />
			<RestartConsent required={access.needsRestart} checked={restartSolver} onChange={setRestartSolver} />
			<button type="submit" className="primary" disabled={access.needsRestart && !restartSolver}>
				Save password
			</button>
			<button type="button" onClick={cancel}>
				Cancel
			</button>
		</form>
	)
}

function SaveRecovery({ code, perform, cancel }: { code: string; perform: Perform; cancel(): void }) {
	const [saved, setSaved] = useState(false)
	const [copyStatus, setCopyStatus] = useState("")
	const copy = async () => {
		try {
			await navigator.clipboard.writeText(code)
			setCopyStatus("Copied")
		} catch {
			setCopyStatus("Select the code and copy it manually.")
		}
	}
	return (
		<form
			onSubmit={(event) => {
				event.preventDefault()
				void perform("confirm-recovery", { saved })
			}}
		>
			<h1>Save your recovery code</h1>
			<p className="hint">
				Keep this code in a password manager or somewhere safe. Use it if you forget your password. Keep it
				private—it can unlock your saved settings.
			</p>
			<span id="recovery-code-label">Your recovery code</span>
			<pre className="code-block mono desktop-recovery-code" aria-labelledby="recovery-code-label" tabIndex={0}>
				<code id="saved-recovery-code">{code}</code>
			</pre>
			<button type="button" onClick={() => void copy()}>
				Copy code
			</button>
			{copyStatus && (
				<p className="hint" role="status">
					{copyStatus}
				</p>
			)}
			<label className="desktop-unlock-choice">
				<input type="checkbox" checked={saved} onChange={(event) => setSaved(event.target.checked)} /> I've
				saved my recovery code
			</label>
			<button type="submit" className="primary" disabled={!saved}>
				Continue
			</button>
			<button type="button" onClick={cancel}>
				Cancel
			</button>
		</form>
	)
}
