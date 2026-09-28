# Desktop config encryption

Closes the implementation gap described in [#1330](https://github.com/polytope-labs/hyperbridge/issues/1330).

Simplex Desktop asks users to create a password and select **Continue** on first use, then requires
authentication on every new Electron launch. Encryption is automatic; users do not manage encryption keys.
Users copy a selectable recovery-code block and confirm saving it before completing setup.
Cancel returns to password creation if setup has not been saved, or to login for an existing profile.
On a fresh launch, valid `desktop-vault.json` metadata selects login; absent metadata selects password
creation unless the config is already encrypted, in which case restoring missing metadata is required.
Reopening a hidden window in the same unlocked Electron session does not require another login.
Supported Macs can additionally enroll Touch ID;
Windows and Linux show password and recovery-code options only (no Windows Hello integration).
The native login gate blocks solver APIs and privileged menu actions until unlock. Web/PWA behavior
is unchanged, including plaintext CLI config storage.

Desktop owns `<userData>/filler-config.toml`, encrypted with AES-256-GCM and a random 32-byte key.
`desktop-vault.json` version 2 stores a scrypt password-wrapped key (N=131072, r=8, p=1; random
16-byte salt). Touch ID authorizes access to an optional macOS Keychain-backed `safeStorage` wrapper;
the wrapper is not itself biometric-bound. A random 256-bit recovery code separately wraps the same
config key; the raw code is never persisted by the app. Version 1 profiles enroll recovery after
successful authentication. Both files use atomic private writes. Backups require both files plus
the password or recovery code. Refresh metadata backups after recovery; old credentials may still
unlock old backups even after rotation.

**Forgot password?** verifies a recovery code or previously enabled Touch ID without requiring the
old password. A ten-minute, in-memory authorization permits choosing a new password and saving a
replacement recovery code. APIs remain locked until the user confirms saving that code; cancellation
before confirmation leaves the existing credentials unchanged. Confirmation atomically replaces the
credential wrappers without changing the config key or restarting a protected solver. If solver
startup subsequently fails, the committed new credentials remain usable and startup can be retried.
Recovery never deletes the profile; users without a recovery method cannot bypass authentication.

The solver's hidden `--config-key-stdin` option requires `--ui-socket` and `--data-dir`, rejects
`--config`, and accepts exactly 32 key bytes over a one-shot pipe. Keys never travel in argv or env.
Protected mode bypasses CLI config discovery and injects the same encrypted writer into setup and
runtime edits. `/health` reports `configEncrypted: true` so desktop cannot attach to a legacy writer.

Existing profile plaintext is encrypted only after any legacy solver is stopped with explicit
consent. Metadata is persisted before migration, so interruption can resume with the password.
Working-directory and `SIMPLEX_HOME` configs are not auto-imported; see the desktop README for migration.
Old plaintext copies/snapshots are not erased. A detached running solver retains its in-memory key
after UI quit; encryption does not secure running process memory, database files, or an authorized tunnel.
