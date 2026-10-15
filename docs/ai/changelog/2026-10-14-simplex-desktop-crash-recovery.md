# Simplex Desktop crash recovery and error reporting

The Simplex web UI and the Simplex Desktop main process report errors to Sentry. Only builds
that CI gives a DSN report: desktop releases and the npm package. Local builds, validation
desktop builds and Docker images bake no DSN and send nothing.

## Build inputs

| Variable | Read by | Effect |
|----------|---------|--------|
| `SIMPLEX_UI_SENTRY_DSN` | `ui/vite.config.ts`, desktop `tsup.config.ts` | Bakes the UI DSN as `__SIMPLEX_UI_SENTRY_DSN__`. The desktop build derives the renderer CSP's single allowed ingest origin from it. |
| `SIMPLEX_DESKTOP_SENTRY_DSN` | desktop `tsup.config.ts` | Bakes the main-process DSN as `__SIMPLEX_DESKTOP_SENTRY_DSN__`. |
| `SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SIMPLEX_UI_SENTRY_PROJECT` | `ui/vite.config.ts` | When all three are set, `@sentry/vite-plugin` uploads hidden UI source maps and deletes them from `dist/ui`. Without them, the UI build emits no maps. |

Releases are `simplex-ui@<version>` and `simplex-desktop@<version>`. The desktop main process
needs no upload: `dist/main.js.map` ships in the asar. The app enters through
`dist/bootstrap.js`, which enables source maps before dynamically loading `main.js`.

## Runtime opt-out

`SIMPLEX_ERROR_REPORTING=off` (also `0`, `false`, `no`):

- The solver's `/health` adds `errorReporting: false`. The UI waits for `/health` before
  initializing Sentry, including session reporting. Opt-out and failed probes leave the SDK
  inactive; a locked desktop's `423` allows initialization because the shell enforces opt-out.
- Desktop main skips `Sentry.init` and leaves Sentry out of the renderer CSP. With no ingest
  origin in `connect-src`, the window cannot report.

## Scrubbing

`@hyperbridge/simplex/error-reporting` (`src/config/error-reporting.ts`) is shared by both
processes. `scrubEvent` and `scrubBreadcrumb` rewrite these fields:

- the message, `logentry`, exception values, breadcrumbs and `extra`;
- `request.url`. Request data, cookies, the query string and every header except `User-Agent`
  are dropped.

The text and recursive scrubbers replace with `[redacted]`:

- 12 or more consecutive BIP-39 words, regardless of case, including serialized or structured word arrays;
- tokens of 32 or more characters (keys, hashes, base58 and base64 secrets, long file paths),
  except 0x addresses;
- absolute Windows, UNC and POSIX file paths, including quoted paths with spaces;
- the path and query of non-loopback URLs.

Stack frames, event ids and trace ids are left alone so grouping and source maps still work.

The desktop removes the `SentryMinidump`/`ElectronMinidump` (process memory),
`LocalVariables*` (frame values), `Screenshots`, `PreloadInjection` and
`RendererEventLoopBlock` integrations. It uses `IPCMode.Classic` and skips OpenTelemetry. The
renderer reports through the UI's own `@sentry/react`, not through an Electron preload.

## Crashes without minidumps

The desktop collects no minidumps, so three reports stand in for them:

- **Child processes:** `childProcessIntegration({ events })` sends an event when a renderer, GPU
  or utility process exits as `crashed`, `oom`, `abnormal-exit`, `launch-failed` or
  `integrity-failure`. Sentry's default sends `crashed` and `oom` only with minidumps.
- **Solver:** `reportSolverFailure` fires where the "Simplex solver stopped" notification does
  (`shouldNotifySolverFailure`). It is tagged with the solver's previous state and version.
- **Main process:** `session-marker.ts` writes `running-session.json` to userData at launch and
  removes it on `will-quit`. A marker left at the next launch is reported as a `fatal` "ended
  without a clean quit", tagged with the previous version. A marker older than the OS boot time
  (`os.uptime()`) is ignored, because a shutdown or restart ended that session. Windows does not
  always emit `will-quit` at shutdown. The marker is only written while reporting is on.

## CI

`publish-simplex.yml` passes the UI variables to the npm build. `publish-simplex-desktop.yml`
passes all of them to the prepare build, but only when `sign_artifacts` is true.
`SIMPLEX_UI_SENTRY_DSN`, `SIMPLEX_DESKTOP_SENTRY_DSN` and `SENTRY_AUTH_TOKEN` are repository
secrets. `SENTRY_ORG` and `SIMPLEX_UI_SENTRY_PROJECT` are repository variables.

## Local diagnostics and recovery

Desktop health, status and control requests reject interrupted responses, enforce an absolute
deadline, and cap responses at 64 KiB. A failed request no longer leaves supervision permanently
waiting on the same promise. Renderer API streams retain their existing streaming behavior.

A renderer crash reloads the dashboard once per window. Subsequent crashes offer an explicit
retry; **Show Simplex** can also reload a crashed renderer. Recovery waits for Chromium teardown,
stops during app exit or update installation, and never restarts the detached solver.
If the renderer dies during initial navigation, its rejected page load does not quit the desktop;
the same recovery handles it. Other initial page-load errors remain fatal startup errors.

OS secure-storage availability checks share one native request and return unavailable after five
seconds. A stalled check cannot indefinitely hide the password login form or accumulate native
requests on retries. OS-protected background keys and passkeys remain unavailable until the native
check completes; password-protected config storage remains encrypted.

Unexpected solver disappearance triggers the native alert and solver error report on every OS,
including Windows, where process exit removes the named pipe. Accepted desktop dashboard stop
requests and native stop actions remain quiet, as does a shutdown observed in the stopping state.
A stop initiated elsewhere that completes between probes can still appear unexpected.

Local diagnostics live under `<userData>/diagnostics`, alongside the existing `logs` directory.
Desktop and solver diagnostics each retain five files, at most 1 MiB per file; individual details
are limited to 16,384 characters. Files are created mode `0600` on Unix and use the shared secret scrubber
before writing. They remain enabled when remote error reporting is off. **Open Data Directory**
provides access; these files are not uploaded as attachments.

The desktop bundles `solver-diagnostics.js` and packages it outside the asar as
`desktop/solver-diagnostics.mjs`. Node loads it with `--import` before loading the solver. It
records handled `console.error` failures and observes fatal exceptions and unhandled rejections
without suppressing Node's failure exit. The shell records child exit codes/signals while it owns
the child handle. Writes are synchronous so an immediate `process.exit` preserves the event;
logging failures do not stop filling. Solver stdio remains detached from Electron.

Native aborts, operating-system kills and power loss cannot run JavaScript diagnostics. No raw
stderr capture or memory dumps are added; OS crash records are still needed for those failures.
The existing Sentry session marker can report an unclean desktop exit on a later launch when
remote reporting is enabled. These changes cannot recover evidence from an older installed build.

## Validation

Desktop CI tests the branch build on macOS, Windows and Linux (x64 and arm64).
The optional `test_published_windows` workflow input also installs the published Windows
v0.17.4 release and runs password-reopen, passkey and reload/reconnect reproduction tests.
Those baseline tests do not validate fixes absent from the published release.
