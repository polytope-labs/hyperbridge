# Simplex: Sentry error reporting

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
needs no upload: `dist/main.js.map` already ships in the asar, and
`process.setSourceMapsEnabled(true)` maps stacks before they are sent.

## Runtime opt-out

`SIMPLEX_ERROR_REPORTING=off` (also `0`, `false`, `no`):

- The solver's `/health` adds `errorReporting: false`. The UI's `beforeSend` waits for
  `/health` and drops every event unless it allows reporting. A failed probe drops events too,
  but a locked desktop's `423` does not.
- Desktop main skips `Sentry.init` and leaves Sentry out of the renderer CSP. With no ingest
  origin in `connect-src`, the window cannot report.

## Scrubbing

`@hyperbridge/simplex/error-reporting` (`src/config/error-reporting.ts`) is shared by both
processes. `scrubEvent` and `scrubBreadcrumb` rewrite these fields:

- the message, `logentry`, exception values, breadcrumbs and `extra`;
- `request.url`. Request data, cookies, the query string and every header except `User-Agent`
  are dropped.

`scrubText` replaces with `[redacted]`:

- 12 or more consecutive BIP-39 words;
- tokens of 32 or more characters (keys, hashes, base58 and base64 secrets, long file paths),
  except 0x addresses;
- the path and query of non-loopback URLs.

Stack frames, event ids and trace ids are left alone so grouping and source maps still work.

The desktop removes the `SentryMinidump`/`ElectronMinidump` (process memory),
`LocalVariables*` (frame values), `Screenshots`, `PreloadInjection` and
`RendererEventLoopBlock` integrations. It uses `IPCMode.Classic` and skips OpenTelemetry. The
renderer reports through the UI's own `@sentry/react`, not through an Electron preload.

## CI

`publish-simplex.yml` passes the UI variables to the npm build. `publish-simplex-desktop.yml`
passes all of them to the prepare build, but only when `sign_artifacts` is true.
`SIMPLEX_UI_SENTRY_DSN`, `SIMPLEX_DESKTOP_SENTRY_DSN` and `SENTRY_AUTH_TOKEN` are repository
secrets. `SENTRY_ORG` and `SIMPLEX_UI_SENTRY_PROJECT` are repository variables.
