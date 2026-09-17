# Simplex Desktop

Private Electron shell for the existing `@hyperbridge/simplex` UI and solver. It does not build a
second renderer: Chromium loads `simplex://local/`, whose protocol handler streams requests to the
solver's Unix socket or Windows named pipe.

## Run from this package

These commands are intentionally package-local. Open a terminal in the directory containing this
README and `package.json`; its parent directory name is not part of the usage contract. pnpm
discovers the workspace above it and resolves the other packages by name.

Use Node 24 to install and build the workspace:

```sh
pnpm install
pnpm --filter @hyperbridge/sdk build
pnpm --filter @hyperbridge/simplex build
pnpm stage:node
pnpm dev
```

The two filtered builds prepare this package's workspace dependencies. The remaining commands are
the desktop package's own scripts. After the first build, normal development launches only need:

```sh
pnpm dev
```

Run `pnpm stage:node` again only when the pinned runtime or the host target changes.

## First run and configuration

With no existing config, `pnpm dev` opens the setup wizard. Complete onboarding normally; the wizard
writes `filler-config.toml` and all desktop runtime data under Electron's
`app.getPath("userData")`. On Unix, a newly written config is mode `0600`.

Config discovery preserves the CLI precedence:

1. `filler-config.toml` in the process working directory;
2. `$SIMPLEX_HOME/config.toml`;
3. `filler-config.toml` in the Electron user-data directory.

An existing config found in either legacy location is used in place and is not copied. Consequently,
a config in this package directory or a configured `SIMPLEX_HOME` suppresses the first-run wizard.

For a disposable profile, pass an explicit Chromium user-data directory:

```sh
pnpm dev -- --user-data-dir=/absolute/path/to/a/disposable-profile
```

This flag is optional. Omit it to use the normal desktop profile and onboarding flow.

## Process lifecycle

Electron attaches to one solver address derived from its user-data directory. If no recognized
Simplex process answers `/health`, it launches the existing Simplex binary with the staged runtime.
It refuses to replace a live, unrecognized listener.

Closing the window hides it to the system tray and intentionally leaves the solver running so UI
lifecycle cannot interrupt in-flight fills. A later desktop launch attaches to that same solver
instead of spawning another, and a second concurrent launch focuses the first window.

The tray and application menus deliberately provide two separate exit commands:

- **Quit Simplex (solver keeps filling)** closes only Electron;
- **Stop solver and quit** requests a graceful solver stop before closing Electron.

The app polls the private socket every three seconds. Its tray icon, native menu, tooltip, and window
title distinguish setup, running, paused, stopping, stopped, and unreachable states. It requires two
consecutive failed probes before declaring a running solver unreachable. A transport failure raises
an operating-system notification; clean socket removal is shown as stopped without calling it a
crash, because it may be a deliberate dashboard stop. Restart is operator-controlled and remains
disabled while graceful shutdown drains in-flight work, so the shell cannot start a second filler on
the same signer.

While the solver is actively filling and Electron remains open, its `powerSaveBlocker` prevents app
suspension. The native menu says whether sleep prevention is on. Pausing or stopping the solver
releases it. This protects an active session from ordinary system sleep, but it cannot survive logout
or guarantee 24/7 laptop uptime. Explicitly quitting Electron releases this protection even though
app-only quit leaves the detached solver running.

## Tray and operating-system integration

The app and tray icons are rasterized from the existing Simplex PWA `mobile-logo.svg`; the tray adds
a small status marker rather than introducing independent artwork. It appears in the macOS menu bar,
the Windows notification area, and Linux's StatusNotifierItem/Gtk status-icon implementation. GNOME
normally requires an AppIndicator extension. Because Linux click activation is inconsistent, every
command—including Show, Pause, Stop, Restart, and both quit choices—is available from the context
menu.

**Launch Simplex at login** is opt-in and available only in an installed build. It registers the app,
not the detached solver, and starts it without opening a window. macOS and Windows use Electron's
login-item API; Linux uses the equivalent per-user XDG autostart entry. Development runs do not
register the Electron development binary.

Native menus also provide About, Open Data Directory, Open Current Log, update checks, and a
stable/beta channel selector while preserving the platform Edit and Window roles and their keyboard
shortcuts. Update controls are available only in an installed build; development launches never
contact the release feed.

Simplex does not upload crash reports. Solver diagnostics remain in rotating NDJSON launch logs under
`<userData>/logs`; five launches are retained, and **Open Current Log** opens the newest one. This
avoids a remote crash-reporting path that could accidentally capture config contents or key material.

Desktop removal leaves Electron's user-data directory in place. Neither this shell nor its installer
contract deletes operator databases, logs, or configuration.
Operators may remove that directory manually only after confirming no reclaimable deposits or records
are needed. The Windows NSIS uninstaller explicitly disables app-data deletion; macOS and Linux
removal likewise leave the per-user data directory untouched.

For genuinely continuous uptime, run `polytopelabs/simplex` on a VPS and use the authenticated tunnel
for remote viewing. Desktop login startup does not make a laptop a server.

## Security model

Desktop Simplex serves the dashboard only on its Unix socket or Windows named pipe. The Electron
launch command never supplies `--ui`, so configuration changes—including enabling the opt-in remote
access tunnel—cannot make the desktop solver open a TCP listener. Tunnel connections are outbound
and their authenticated channels are injected directly into the socket-backed UI server.

The renderer is sandboxed, has context isolation and web security enabled, and has no Node
integration, preload bridge, IPC API, or webview support. Production builds disable DevTools. A
desktop-only Content Security Policy limits scripts and API connections to `simplex://local`; all
browser permissions are denied except sanitized clipboard writes from that exact origin. Navigation
cannot leave that origin. Links opened with `target=_blank` are denied an Electron child window and
only the UI's exact HTTPS explorer/HyperFX hosts may be handed to the operating system browser.

On Unix, the UI socket and newly generated config are mode `0600`. Windows named pipes do not offer
an equivalent portable owner-only guarantee through Node's current APIs; administrators and some
same-machine contexts may still be able to inspect or connect to the pipe. On every platform,
software already running as the same OS user remains inside the trust boundary.

`filler-config.toml` contains private signing material in plaintext. Simplex writes new files
atomically with a prominent warning and mode `0600` on Unix, but operators must still keep the file
out of source control and broadly shared backups. Desktop does not copy keys into renderer storage,
logs, or crash-report uploads, and it does not configure a crash-report uploader.

The CLI's normal `127.0.0.1` web UI is a machine-local, unauthenticated interface—not a per-user
boundary. On a shared machine, another local OS user may be able to reach it and invoke operator
actions. Prefer `--ui-socket` with owner-only filesystem permissions, or the authenticated opt-in
tunnel, whenever other users are not trusted. Never bind the CLI UI to a non-loopback interface
without a separate trusted network boundary.

## Runtime resources

The staging script follows Node's binary-verification procedure: it verifies the signed
`SHASUMS256.txt.asc` with the pinned release key, verifies the selected Node 24.19.0 archive checksum,
and installs only `node` or `node.exe`. `darwin-universal` verifies both macOS slices independently
before merging them with `lipo`.

The desktop process requires the three solver resources plus the status variants derived from the
PWA logo, and fails before creating a window if any is missing:

- `resources/node/<platform>-<arch>/node` in development, or `node.exe` on Windows;
- `@hyperbridge/simplex/dist/bin/simplex.js`;
- `@hyperbridge/simplex/dist/ui/index.html`.
- `resources/tray/<state>.png` in development, with 18px macOS `Template` and 36px `Template@2x`
  variants; packaged builds place them under `desktop/tray` in Electron resources.

It never searches `PATH` for the solver runtime. The release build keeps the Electron main process in
`app.asar` and places the solver's `package.json` and `dist` tree under `resources/simplex`, its
production dependency closure under `resources/node_modules`, and the independently verified Node
executable under `resources/runtime`. The runtime dependency workspace is locked separately and
contains the six packages intentionally left external by the Simplex bundle: `ssh2`, `pino`,
`pino-pretty`, `thread-stream`, `@solana/web3.js`, and `@solana/spl-token`.

## Build installers locally

Install the release-only tools from this package directory. They use a separate lockfile so
`electron-builder` and its platform packagers do not alter the SDK dependency graph:

```sh
pnpm --dir tooling install --frozen-lockfile
```

Then build the SDK, solver, and desktop main process, stage the verified host runtime, and package the
current platform:

```sh
pnpm --filter @hyperbridge/sdk build
pnpm --filter @hyperbridge/simplex build
pnpm build
pnpm stage:node
pnpm package -- --mac --arm64 --publish never
```

Replace the final target arguments with `--mac --x64`, `--win --x64`, `--linux --x64`, or
`--linux --arm64` on a matching native host. The output is written to `release/`. Smoke-test its
unpacked application, each generated artifact payload, and enforce the installed-size budget with:

```sh
pnpm package:smoke -- --root release
pnpm package:smoke:artifacts -- --root release
node scripts/package-size.mjs --root release --budget-mib 520
```

The first complete macOS arm64 package measures about 495 MiB; Electron's framework alone accounts
for about 287 MiB. Windows x64 measures about 550 MiB. CI therefore enforces target-specific
installed-size budgets: 520 MiB for macOS and Linux, and 580 MiB for Windows. Each budget leaves
roughly five percent growth headroom while still catching accidental duplication.

Pull requests that change desktop packaging run the complete native matrix before merge. Pushing the
exact package-version tag, for example `simplex-desktop-v0.16.2`, runs the same matrix for macOS arm64
and x64 DMG plus updater ZIP, Windows x64 NSIS, and Linux x64 and arm64 AppImage plus deb. Pull-request
builds and default manual runs are explicitly unsigned. A signed manual run or tag build instead fails
before packaging unless its native signing environment is complete; an unsigned release artifact can
never be used as a fallback.

The workflow smoke-tests each unpacked application and each artifact through its private socket. It
mounts the macOS DMG, extracts the updater ZIP, silently installs NSIS and deb packages, and launches
the AppImage executable (using its self-extract runtime only when hosted-runner FUSE is unavailable);
local Linux runs extract the deb instead of modifying the host. It verifies the packaged setup UI and
also launches the packaged Node/solver pair with captured stderr so startup warnings fail the build.
It drives packaged onboarding through config creation and a fail-closed offline boot, recomputes every
updater SHA-512, and validates the exact asset set. The macOS jobs additionally require the app,
Electron helpers, bundled Node, and DMG to have the expected Developer ID team, the app, helpers, and
Node to have exactly the two JIT entitlements, the app and DMG to pass Gatekeeper assessment, and the app to have a stapled
notarization ticket. The signed DMG is separately submitted to `notarytool` as the outer distribution
container and must also carry a valid stapled ticket. The Windows job
requires valid, timestamped Authenticode signatures from the configured publisher on every packaged
executable, including `Simplex.exe`, the bundled `node.exe`, and the NSIS installer.

Only after every native job passes does CI publish the GitHub release. A failed upload remains a draft,
and CI refuses to mutate an already-public release. Desktop tags are separate from `simplex-v*`, so
they do not publish npm or Docker artifacts. Linux packages are public for manual installation, but
Linux automatic updates remain disabled until channel metadata has an independent signature.

### Release signing configuration

macOS releases use a Developer ID Application certificate, hardened runtime, and Apple's `notarytool`.
The app and bundled Node runtime receive only
`com.apple.security.cs.allow-jit` and
`com.apple.security.cs.allow-unsigned-executable-memory`; automatic entitlement expansion is disabled.
Store these as secrets in a GitHub Actions environment named `simplex-desktop-release`:

- `SIMPLEX_MACOS_CERTIFICATE_P12`: base64-encoded Developer ID Application `.p12`;
- `SIMPLEX_MACOS_CERTIFICATE_PASSWORD`: export password for that `.p12`;
- `SIMPLEX_APPLE_API_KEY_P8_BASE64`: base64-encoded App Store Connect API `.p8` key;
- `SIMPLEX_APPLE_API_KEY_ID`, `SIMPLEX_APPLE_API_ISSUER`, and `SIMPLEX_APPLE_TEAM_ID`.

Windows releases use Azure Trusted Signing. Store its workload identity as secrets in the same
`simplex-desktop-release` environment:

- `SIMPLEX_AZURE_TENANT_ID`;
- `SIMPLEX_AZURE_CLIENT_ID`;
- `SIMPLEX_AZURE_CLIENT_SECRET`.

Store the non-secret Trusted Signing resource identity as variables in that environment:

- `SIMPLEX_WINDOWS_PUBLISHER_NAME`, exactly matching the certificate's simple subject name;
- `SIMPLEX_AZURE_SIGNING_ENDPOINT`, an HTTPS `*.codesigning.azure.net` endpoint;
- `SIMPLEX_AZURE_SIGNING_ACCOUNT_NAME`;
- `SIMPLEX_AZURE_CERTIFICATE_PROFILE_NAME`.

Configure the `simplex-desktop-release` environment to allow only the `main` branch and tags matching
`simplex-desktop-v*`, and require release-maintainer approval. Unsigned builds use a separate,
secretless `simplex-desktop-ci` environment. The workflow never uses signing secrets for
`pull_request`, including fork pull requests. A manual
dispatch is unsigned by default; a maintainer can explicitly enable `sign_artifacts` on `main` to
exercise the complete credentialed pipeline and download its private workflow artifacts without
publishing a release. Credentialed dispatches from other refs fail before any secret-bearing step. A
pushed `simplex-desktop-v*` tag always enables signing and is the only event that publishes; CI also
requires the tagged commit to belong to `origin/main`. Protect this tag namespace with a repository
ruleset so only release maintainers can create or delete matching tags. Keep the signing values out of
repository-level secrets: environment branch/tag rules cannot protect repository secrets.
To rotate credentials, provision the replacement at Apple or Microsoft first, update the corresponding
environment secrets (and variables if the Azure resource identity changed), run a signed manual dispatch,
and revoke the old certificate, API key, or service-principal secret only after both native signature
jobs and clean-machine installation checks pass. Never reuse or move an existing release tag.

Apple Developer Program enrollment, creation of the Developer ID certificate and App Store Connect
key, and creation of the Azure Trusted Signing account/profile are operational prerequisites. Download
the artifacts from a signed manual dispatch, install the DMG on a clean macOS account and the NSIS
installer on a clean Windows VM, complete onboarding with live credentials, and perform a real fill
before creating the public tag. The macOS CI job already runs `spctl`; the Windows VM check is still
required to observe SmartScreen reputation, which cannot be established by inspecting an
Authenticode signature alone.

Once the secrets and variables exist on the repository, start the private validation run from the
merged revision with:

```sh
gh workflow run publish-simplex-desktop.yml --ref main -f sign_artifacts=true
```

Download the `simplex-desktop-darwin-*` and `simplex-desktop-win32-x64` artifacts from that run for
the clean-machine checks. On the Mac, verify the installed copy with
`spctl --assess --type execute --verbose=4 /Applications/Simplex.app`. On Windows, retain a screenshot
of the SmartScreen result and use `Get-AuthenticodeSignature` on every installed `.exe`, including
the installer, `Simplex.exe`, and `resources\runtime\node.exe`; each must report `Valid`, the
configured publisher, and a timestamp.

## Updates and rollback

Trusted installed builds check the `simplex-desktop-v*` releases in `polytope-labs/hyperbridge` at
launch and every six hours. On macOS the installed application must pass the operating system's code
signature verification. On Windows the packaged updater configuration must contain a non-empty
Authenticode publisher identity. Linux automatic updates remain disabled until release metadata has
an independent signature. Unsigned packages therefore cannot download or install updates. Stable is
the default channel and beta is opt-in from the native menu when updates are enabled.

The provider accepts only plain artifact filenames and resolves them under the selected HTTPS GitHub
release directory. Absolute URLs, foreign hosts, path traversal, query strings, and fragments are
rejected. SHA-512 still protects download integrity, but is not treated as publisher authentication
because the checksum and artifact list come from the same release metadata.

After download, the app waits until `/api/status` reports no active evaluation, queued fill, active
fill, bid retraction, or portfolio rebalancing work. A running solver must also have no queued
evaluations; a paused solver may discard those not-yet-started evaluations through its existing
graceful-stop behavior. The app then asks the solver to stop gracefully and waits for both its private
socket and operating-system process to disappear before invoking the installer. A solver that does
not drain is never killed: the update remains staged and is retried. After 24 hours the app notifies
the operator to pause new fills and create a safe window.

A previous-version solver that does not yet report its PID can still be attached and restarted
gracefully, using release of the private socket as the exit proof. Automatic installation remains
staged until the app is supervising a PID-reporting solver, so updater safety is never weakened.

A staged update is also left untouched while the operator has intentionally stopped the solver. On
relaunch, Electron rechecks the feed so its updater instance revalidates the cached artifact before
stopping anything. If the installer reports an error after the updater stopped the solver, the app
clears the attempted marker, restarts that solver, and leaves the update available for a later retry.

Before installing, the app records the old and target versions in `desktop-updates.json` under
Electron user data. The relaunched app clears that receipt only after a healthy setup or operator
solver reports the same version as the desktop app. A solver boot failure uses the native startup
error and exits instead of failing silently. A version mismatch remains visible in the native menu
and blocks both onboarding and the dashboard from driving the mismatched solver. Changing update
channels ignores a download that was started on the previous channel.

Update artifacts use electron-updater's SHA-512 metadata checks after the platform trust gate passes.
macOS updates require a valid application signature and Windows NSIS updates require a configured
Authenticode publisher. The builder configuration fixes the GitHub owner and repository, while
`SimplexReleaseProvider` fixes the `simplex-desktop-v` tag prefix and the exact release download path.

Rollback is manual once signed releases are public. Stop the solver gracefully, download the previous
signed installer from GitHub Releases, and install it over the current build. Previous releases and
their update metadata must remain downloadable. Switching from beta to stable does not downgrade to
a numerically older stable version automatically.

## Verification

Run the fast checks from this directory:

```sh
pnpm typecheck
pnpm lint
pnpm test
```

After building Simplex and staging the host runtime as shown above, run the real Electron suite:

```sh
pnpm test:e2e
```

The E2E suite closes and hard-kills Electron to prove the detached solver survives and is reused,
launches a second copy to prove it focuses the first without spawning, verifies native lifecycle
state and sleep prevention, exercises pause/resume and crash/restart, checks that the solver has no
TCP listener and writes a disk log, restarts a real `UiServer` behind the custom protocol, checks CSP
and external-navigation enforcement, verifies one SSE client remains after twenty reloads, and
verifies first-run config placement, secret-storage hygiene, and Unix mode `0600`. It uses unreachable
loopback endpoints for the first-run write and does not start a real filler.
