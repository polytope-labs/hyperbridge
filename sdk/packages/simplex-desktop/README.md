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

Closing or crashing Electron intentionally leaves the solver running so UI lifecycle cannot interrupt
in-flight fills. A later desktop launch attaches to that same solver instead of spawning another.
Use the dashboard's explicit **Stop** action when the solver itself should exit.

Continuous crash supervision, tray integration, login startup, and focus restoration are deferred.
If the solver exits while Electron remains open, this shell does not automatically restart it.

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

The desktop process requires all three resources and fails before creating a window if any is
missing:

- `resources/node/<platform>-<arch>/node` in development, or `node.exe` on Windows;
- `@hyperbridge/simplex/dist/bin/simplex.js`;
- `@hyperbridge/simplex/dist/ui/index.html`.

It never searches `PATH` for the solver runtime. Packaging, signing, installers, updates, and final
packaged resource placement are intentionally outside this package's current scope.

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

The E2E suite hard-kills and relaunches Electron to prove the detached solver survives and is reused,
checks that the solver has no TCP listener, restarts a real `UiServer` behind the custom protocol,
checks CSP and external-navigation enforcement, verifies one SSE client remains after twenty reloads,
and verifies first-run config placement, secret-storage hygiene, and Unix mode `0600`. It uses
unreachable loopback endpoints for the first-run write and does not start a real filler.
