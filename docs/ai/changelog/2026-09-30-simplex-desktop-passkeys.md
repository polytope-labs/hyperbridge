# Simplex Desktop passkey login

macOS and Windows offer passwordless first-run setup and login using WebAuthn platform passkeys
through the system browser. Password setup remains available as a small **Use a password instead**
text action below the passkey controls; Linux retains password login.
Existing password profiles enroll with **Create a passkey for future logins** during authenticated
login, preserving their password fallback, recovery code and config key.

The host verifies registration/authentication with required user verification, a fresh one-use
challenge, exact expected origin, RP ID `localhost`, credential ID and assertion signature/counter.
A temporary listener binds only `127.0.0.1` on a random port. Its browser page authenticates requests
with a random capability carried in the URL fragment and then removed from the address bar.
It exposes only the ceremony and closes on completion, cancellation, failure or the two-minute
expiry. The desktop API remains locked until verification and solver activation succeed.

`desktop-vault.json` version 2 password profiles remain readable. Passkey profiles use version 3
and persist `passkey.id`, base64url COSE `passkey.publicKey`, `passkey.counter`, optional transports
and an Electron `safeStorage` key wrapper in `passkey.key`. New passkey profiles omit `salt` and
`wrappedKey`; enrolled password profiles retain them. Both formats retain recovery and optional
background-resume wrappers. Passkeys authorize app access; encryption still uses the existing
OS-protected random config key, so a synced passkey alone cannot restore a profile on another
machine or OS account. Such restores require both files and the recovery code.

**Recover access** verifies the recovery code before allowing a replacement
passkey or password. The replacement recovery code must be acknowledged before committing the
change. Password recovery removes the current passkey; passkey replacement retains any existing
password fallback. Cancelling preserves the saved profile. Recovery keeps the config key and an
already protected solver running; old metadata backups may still unlock with their old wrappers.
The standalone Touch ID option is removed: macOS users get Touch ID through passkeys. Existing
profiles load unchanged, ignore their `biometricKey` Touch ID wrapper, drop it on the next save,
and unlock with their password or recovery code.

Desktop security state adds `passkeyAvailable`, `passkeyEnabled` and `passwordEnabled` and drops
`biometricAvailable`/`biometricEnabled`; unlock no longer accepts `useBiometrics` or `method: "biometric"`.
`POST /api/desktop/unlock` accepts `create-passkey`, `passkey` and password enrollment via
`usePasskey: true`; recovery accepts `method: "passkey"`. Authenticated recovery uses
`POST /api/desktop/reset-passkey`; `POST /api/desktop/cancel-passkey` cancels the active ceremony.
Both routes enforce the existing native-UI origin and CSRF checks. The existing `reset-password`
mode represents authorized credential replacement, including passkeys.
