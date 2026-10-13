# Simplex macOS native passkeys

Mac desktop authentication uses an Objective-C++ Node-API bridge to Apple's
AuthenticationServices, attached to the Simplex window. Native Mac ceremonies do
not open a browser or create a loopback HTTP listener. Windows retains its browser
ceremony; Linux retains password access.

The host verifies registration/assertion bytes with SimpleWebAuthn before releasing
the existing OS-wrapped configuration key. The wallet signer and automatic solver
resume policy remain unchanged. A native passkey is an app access credential;
safeStorage decryption is not itself guaranteed to require biometric presence.

## Apple setup required for a signed build

Use the permanent bundle ID `network.hyperbridge.simplex` and RP hostname
`passkeys.hyperfx.finance`, configured in the desktop package's
`resources/passkeys.json`. Enable Associated Domains for its Apple App ID. Create
a Developer ID distribution provisioning profile that authorizes the capability;
a development profile is appropriate only for separate local development signing.

The `simplex-desktop-release` GitHub environment needs:

| Setting | Kind | Value |
| --- | --- | --- |
| `SIMPLEX_MACOS_PROVISIONING_PROFILE_BASE64` | Secret | Base64 encoding of the Developer ID distribution `.provisionprofile`. |

Existing Apple certificate, Team ID, keychain and notarization secrets are reused.

Local signed packaging uses `APPLE_TEAM_ID` and
`SIMPLEX_MACOS_PROVISIONING_PROFILE_PATH`, alongside the existing signing/notary
environment. From the desktop package, `pnpm prepare:passkeys` validates the profile
and writes `build/mac-passkeys/apple-app-site-association` and the main app's generated
entitlements. Profile validation checks expiry, platform, distribution mode,
application identifier, team, and associated-domain authorization.

Host that exact association file at
`https://passkeys.hyperfx.finance/.well-known/apple-app-site-association` with valid
HTTPS, `Content-Type: application/json`, and no redirect. Its `webcredentials.apps` uses the actual profile-authorized
application identifier; its prefix is not inferred from the signing Team ID.
Apple association caching can delay initial availability.

`SIMPLEX_PASSKEY_RP_ID` can explicitly override the hostname for a separate build
environment. Development, packaging, and release verification share the checked-in
default; packaged apps use their signed metadata. Keep the release RP ID permanent:
changing it requires enrolling new credentials and hosting a matching association.

Packaging embeds the authorized profile and restricts identity/domain entitlements
to the main app. Helpers and the solver keep their existing entitlements. The
release verifier checks entitlement values, the profile, native module signatures,
Gatekeeper, and notarization. Signed builds fail when the passkey identity inputs
are missing. CI uploads the public association file as a separate artifact.

## Development and installation

`pnpm build:native` compiles the Mac module using the pinned Electron headers when
available, or Node-API headers from the installed Node distribution. Set
`NODE_API_INCLUDE` if headers are elsewhere. Packaging compiles and stages the
target slice explicitly; arm64, x64, and universal outputs use macOS 13 as the
deployment target. The module resides outside ASAR and needs no runtime download.

The SDK `test` and `live` jobs validate pull requests using GitHub's proposed merge
commit. Both checkouts therefore include the current `main` package scripts and
test sources referenced by the workflow, even when a feature branch predates them.
Push and manual runs use the triggering commit.

Unsigned or improperly provisioned builds expose a reason and retain password and
recovery access. Native availability is structural readiness, not proof that a
provider contains a matching credential. The actual Mac OS/provider matrix and
signed native sheet remain release acceptance gates.

## Profiles, migration, and cancellation

Native credentials use vault version 4 with a stable `profileId`, explicit passkey
`rpId` and `mechanism: native-macos`, public credential metadata, and the existing
encrypted key wrappers. Versions 2/3 remain readable. Old credentials are scoped to
`localhost`; Mac users enroll a native replacement after password or recovery-code
authorization. An old credential is never relabeled for the new domain.

Native enrollment uses a fresh credential user handle and an authentication proof
before persistence. The UI explains the two Mac prompts. Passwords entries contain
the stable profile ID prefix and enrollment time to distinguish profiles and
replacements. This preserves a previous provider credential if replacement
fails before commit. Recovery-based replacement still requires acknowledgement of
the replacement recovery code. Old/orphan provider entries may remain; replacement
does not revoke old metadata backups. **Do not downgrade after native enrollment.**
Older binaries cannot read v4, including its password and recovery wrappers. The
updater disables downgrades; for a manually downgraded installation, reinstall a
compatible build before using password or recovery.

The configuration key and wallet address do not change during migration. Restoring
on another Mac/account requires both vault/configuration files and recovery; a synced
passkey alone does not restore the local key wrapper.

The host reports `passkeyMechanism`, `passkeyUnavailableReason`,
`passkeyMigrationRequired`, and `passkeyEnrollmentAvailable` in security state.
Passkey status retains `pending` and adds native state/operation/error information.
The Mac waiting screen says **Confirm with your Mac**. Cancellation, timeout, window
hiding/destruction, reload/crash, sleep, and OS lock/session switch cancel pending
passkey results without discarding password or authorized recovery state. Once
verified access reaches solver activation, cancellation is closed and the unlock
finishes, keeping solver and dashboard access consistent. Quit locks the vault.
Ordinary window blur and same-document navigation do not cancel the OS sheet. Resume/unlock
never authenticates automatically.

## Signed release acceptance

Install a signed DMG on disposable Apple silicon and Intel profiles. Create a native
passkey, acknowledge recovery, quit/relaunch, unlock, cancel/retry, replace and
recover. Test legacy password and passkey-only migrations, provider/credential loss,
offline use after enrollment, OS lock/sleep, and a signed update with unchanged
identity. Confirm no browser launch or HTTP ceremony listener on Mac.

Unit verification and native compilation do not prove Apple association, real
provider prompts, or Touch ID. Approving the genuine OS prompt and validating both
hardware targets are required before declaring the feature ready for release.
