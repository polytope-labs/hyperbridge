import { useCallback, useEffect, useState, type ReactNode } from "react"
import { toast } from "sonner"
import { api } from "../api"
import { isNativeDesktopProtocol } from "../lib/runtime"
import { formatRecoveryCode, RECOVERY_CODE_DIGITS } from "../lib/recovery-code"

type AccessState = {
	mode: "create" | "unlock" | "reset-password" | "save-recovery" | "unlocked"
	passkeyAvailable: boolean
	passkeyEnabled: boolean
	passwordEnabled: boolean
	recoveryEnabled: boolean
	backgroundResumeAvailable: boolean
	secureDeviceStorageAvailable: boolean
	needsRestart: boolean
}
type Perform = (route: string, body?: Record<string, unknown>) => Promise<void>

type AccessMethod = "password" | "passkey"

function preferredAccessMethod(access: AccessState): AccessMethod {
	return access.passkeyEnabled || (access.mode === "create" && access.passkeyAvailable) ? "passkey" : "password"
}

/** Requests that may open a browser ceremony; the host reports when one is actually open. */
function mayOpenPasskey(route: string, body?: Record<string, unknown>): boolean {
	return (
		route === "reset-passkey" ||
		body?.method === "passkey" ||
		body?.method === "create-passkey" ||
		body?.usePasskey === true
	)
}

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
	const [passkeyPending, setPasskeyPending] = useState(false)
	const [notice, setNotice] = useState<string>()
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
	const unlocked = access?.mode === "unlocked"
	useEffect(() => {
		// Child effects run first, so the app's Toaster is mounted by the time this fires.
		if (!unlocked || !notice) return
		toast.warning(notice)
		setNotice(undefined)
	}, [unlocked, notice])
	const perform: Perform = async (route, body) => {
		setPending(true)
		setError(undefined)
		let settled = false
		const poll = mayOpenPasskey(route, body)
			? setInterval(() => {
					void api
						.get<{ pending: boolean }>("/api/desktop/passkey-status")
						.then(({ pending }) => {
							if (!settled) setPasskeyPending(pending)
						})
						.catch(() => {})
				}, 400)
			: undefined
		try {
			const result = await api.post<{ warning?: string } | undefined>(`/api/desktop/${route}`, body)
			setNotice(result?.warning)
			setForgot(false)
			await refresh()
		} catch (cause) {
			await refresh().catch(() => {})
			setError(message(cause))
		} finally {
			settled = true
			clearInterval(poll)
			setPending(false)
			setPasskeyPending(false)
		}
	}
	if (unlocked) return children
	const cancel = () => {
		void perform("cancel-recovery")
	}
	return (
		<main className="desktop-unlock">
			<section className="card" aria-label="Simplex Desktop access">
				<div className="desktop-unlock-brand">
					<img src="./icons/mobile-logo.svg" alt="" width="36" height="36" />
					<span className="eyebrow">Simplex Desktop</span>
				</div>
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
					<>
						<fieldset disabled={pending} hidden={passkeyPending}>
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
									key={access.mode}
									access={access}
									perform={perform}
									forgot={() => {
										setForgot(true)
										setError(undefined)
									}}
								/>
							)}
							{pending && !passkeyPending && (
								<p className="hint" role="status">
									Please wait…
								</p>
							)}
						</fieldset>
						{passkeyPending && (
							<div className="desktop-unlock-panel desktop-passkey-waiting">
								<span className="desktop-passkey-spinner" aria-hidden="true" />
								<h1>Continue in your browser</h1>
								<p className="hint" role="status">
									We opened a browser tab to confirm your passkey. Finish there and Simplex continues
									automatically.
								</p>
								<button
									type="button"
									onClick={() =>
										void api
											.post("/api/desktop/cancel-passkey", {})
											.catch((cause) => setError(message(cause)))
									}
								>
									Cancel request
								</button>
							</div>
						)}
					</>
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
	const [usePasskey, setUsePasskey] = useState(false)
	const [selectedMethod, setSelectedMethod] = useState<AccessMethod>()
	const [restartSolver, setRestartSolver] = useState(false)
	const unlock = async (method: string) => {
		await perform("unlock", { method, password, confirmation, usePasskey, restartSolver })
		setPassword("")
		setConfirmation("")
	}
	const passkeyFirst = (selectedMethod ?? preferredAccessMethod(access)) === "passkey"
	if (passkeyFirst)
		return (
			<PasskeyLogin
				access={access}
				restartSolver={restartSolver}
				setRestartSolver={setRestartSolver}
				unlock={unlock}
				choosePassword={() => setSelectedMethod("password")}
				forgot={forgot}
			/>
		)
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
			{!creating && access.passkeyAvailable && !access.passkeyEnabled && (
				<label className="desktop-unlock-choice">
					<input
						type="checkbox"
						checked={usePasskey}
						onChange={(event) => setUsePasskey(event.target.checked)}
					/>
					Create a passkey for future logins
				</label>
			)}
			<RestartConsent required={access.needsRestart} checked={restartSolver} onChange={setRestartSolver} />
			<button type="submit" className="primary" disabled={access.needsRestart && !restartSolver}>
				{creating ? "Continue" : "Unlock"}
			</button>
			<div className="desktop-unlock-links">
				{access.passkeyAvailable && (creating || access.passkeyEnabled) && (
					<button
						type="button"
						className="desktop-password-alternative"
						onClick={() => setSelectedMethod("passkey")}
					>
						Use a passkey instead
					</button>
				)}
				{!creating && (
					<button type="button" className="desktop-password-alternative" onClick={forgot}>
						Forgot password?
					</button>
				)}
			</div>
		</form>
	)
}

function PasskeyLogin({
	access,
	restartSolver,
	setRestartSolver,
	unlock,
	choosePassword,
	forgot,
}: {
	access: AccessState
	restartSolver: boolean
	setRestartSolver(value: boolean): void
	unlock(method: string): Promise<void>
	choosePassword(): void
	forgot(): void
}) {
	const creating = access.mode === "create"
	return (
		<div className="desktop-unlock-panel">
			<h1>{creating ? "Create a passkey" : "Unlock Simplex"}</h1>
			{creating && <p className="hint">Sign in with Touch ID or Windows Hello.</p>}
			{!access.passkeyAvailable && (
				<p className="hint" role="note">
					Passkey or secure key storage is unavailable on this device. Use your recovery code or another
					sign-in method.
				</p>
			)}
			<RestartConsent required={access.needsRestart} checked={restartSolver} onChange={setRestartSolver} />
			<div className="desktop-unlock-actions">
				<button
					type="button"
					className="primary"
					disabled={!access.passkeyAvailable || (access.needsRestart && !restartSolver)}
					onClick={() => void unlock(creating ? "create-passkey" : "passkey")}
				>
					{creating ? "Create passkey" : "Unlock with passkey"}
				</button>
			</div>
			<div className="desktop-unlock-links">
				{(creating || access.passwordEnabled) && (
					<button type="button" className="desktop-password-alternative" onClick={choosePassword}>
						Use a password instead
					</button>
				)}
				{!creating && (
					<button type="button" className="desktop-password-alternative" onClick={forgot}>
						Recover access
					</button>
				)}
			</div>
		</div>
	)
}

function Recover({ access, perform, cancel }: { access: AccessState; perform: Perform; cancel(): void }) {
	const [recoveryCode, setRecoveryCode] = useState("")
	const complete = recoveryCode.replaceAll("-", "").length === RECOVERY_CODE_DIGITS
	return (
		<form
			onSubmit={(event) => {
				event.preventDefault()
				void perform("recover", { method: "code", recoveryCode })
			}}
		>
			<h1>{access.passkeyEnabled ? "Recover access" : "Reset your password"}</h1>
			{access.recoveryEnabled && (
				<>
					<p className="hint">Enter the recovery code you saved during setup.</p>
					<label htmlFor="recovery-code">Recovery code</label>
					<textarea
						id="recovery-code"
						className="desktop-recovery-input mono"
						required
						autoFocus
						rows={3}
						value={recoveryCode}
						placeholder="XXXXXXXX-XXXXXXXX-…"
						autoComplete="off"
						autoCapitalize="characters"
						spellCheck={false}
						onChange={(event) => setRecoveryCode(formatRecoveryCode(event.target.value))}
						onKeyDown={(event) => {
							if (event.key !== "Enter") return
							event.preventDefault()
							if (complete) event.currentTarget.form?.requestSubmit()
						}}
					/>
					<button type="submit" className="primary" disabled={!complete}>
						Continue
					</button>
				</>
			)}
			{!access.recoveryEnabled && (
				<p className="hint">
					Recovery wasn't set up for this profile. Sign in with your password to set it up, or restore a
					backup you can unlock. Your existing data will not be deleted.
				</p>
			)}
			<button type="button" className="desktop-password-alternative" onClick={cancel}>
				Back to login
			</button>
		</form>
	)
}

function NewPassword({ access, perform, cancel }: { access: AccessState; perform: Perform; cancel(): void }) {
	const [selectedMethod, setSelectedMethod] = useState<AccessMethod>()
	const [password, setPassword] = useState("")
	const [confirmation, setConfirmation] = useState("")
	const [restartSolver, setRestartSolver] = useState(false)
	if (access.passkeyAvailable && (selectedMethod ?? preferredAccessMethod(access)) === "passkey")
		return (
			<div className="desktop-unlock-panel">
				<h1>Create a replacement passkey</h1>
				<p className="hint">This replaces your current passkey.</p>
				<RestartConsent required={access.needsRestart} checked={restartSolver} onChange={setRestartSolver} />
				<div className="desktop-unlock-actions">
					<button
						type="button"
						className="primary"
						disabled={access.needsRestart && !restartSolver}
						onClick={() => void perform("reset-passkey", { restartSolver })}
					>
						Create replacement passkey
					</button>
					<button type="button" onClick={cancel}>
						Cancel
					</button>
				</div>
				<button
					type="button"
					className="desktop-password-alternative"
					onClick={() => setSelectedMethod("password")}
				>
					Use a password instead
				</button>
			</div>
		)
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
			{access.passkeyAvailable && (
				<button type="button" onClick={() => setSelectedMethod("passkey")}>
					Use a passkey instead
				</button>
			)}
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
			<p className="hint">Store it somewhere safe, like a password manager. You'll need it if you lose access.</p>
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
