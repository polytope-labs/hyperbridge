# Simplex Desktop

Private Electron shell for the existing `@hyperbridge/simplex` UI and solver. It does not build a
second renderer: Chromium loads `simplex://local/`, whose protocol handler streams requests to the
solver's Unix socket or Windows named pipe.

## Run from this package

These commands are intentionally package-local. Open a terminal in the directory containing this
README and `package.json`; its parent directory name is not part of the usage contract. pnpm
discovers the workspace above it and resolves the other packages by name.

Use Node 22.16 or newer to install and build the workspace:

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

Desktop removal must leave Electron's user-data directory in place. Installer work is separate, but
neither this shell nor its uninstall contract deletes operator databases, logs, or configuration;
operators may remove that directory manually only after confirming no reclaimable deposits or records
are needed.

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

It never searches `PATH` for the solver runtime. Packaged resource placement and installer generation
remain responsibilities of the desktop release build.

## Updates and rollback

Installed builds check the `simplex-desktop-v*` releases in `polytope-labs/hyperbridge` at launch and
every six hours. Stable is the default channel; beta is opt-in from the native menu. Downloads happen
in the background without interrupting the solver. Ordinary app quit never installs a downloaded
update.

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

Before installing, the app records the old and target versions in `desktop-updates.json` under
Electron user data. The relaunched app clears that receipt only after a healthy setup or operator
solver reports the same version as the desktop app. A solver boot failure uses the native startup
error and exits instead of failing silently. A version mismatch remains visible in the native menu
and blocks both onboarding and the dashboard from driving the mismatched solver. Changing update
channels ignores a download that was started on the previous channel.

Update artifacts use electron-updater's SHA-512 metadata checks. macOS updates additionally require
the app's code signature, and Windows NSIS updates retain Authenticode publisher verification. The
release configuration explicitly fixes the GitHub owner, repository, and `simplex-desktop-v` tag
prefix so installed copies cannot silently follow a renamed build repository.

Rollback is manual. Stop the solver gracefully, download the previous signed installer from GitHub
Releases, and install it over the current build. Previous releases and their update metadata must
remain downloadable. Switching from beta to stable does not downgrade to a numerically older stable
version automatically.

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
