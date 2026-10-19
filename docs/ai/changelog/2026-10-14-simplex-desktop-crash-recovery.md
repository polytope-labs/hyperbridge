# Simplex Desktop crash recovery and local diagnostics

The dashboard reloads automatically after its first renderer crash. Later crashes offer
**Reload dashboard** or **Later**; a crash during a requested reload offers a fresh prompt
once that load finishes. **Show Simplex** also retries a crashed dashboard. Recovery waits
for Chromium teardown, stops during app exit or update installation, and leaves the
separate solver process running.

A rejected initial page load waits up to one second for a recoverable renderer-exit event.
This handles either event order; without that event, the original load error remains fatal.

OS secure-storage availability checks share one native request and return unavailable after
five seconds. While storage is unavailable, the locked login screen checks again every two
seconds, with one refresh in flight at a time. Refreshes pause during authentication and stop
on unlock or unmount. A completed native check can restore passkey access without restarting
the app. Password-protected config remains encrypted, and passkeys and background resume
require available OS storage.

Desktop health, status and control requests reject interrupted responses, enforce an absolute
deadline, and cap responses at 64 KiB. Renderer API streams retain their streaming behavior.
Unexpected solver disappearance triggers the native alert on every OS, including removal of
a Windows named pipe. Accepted desktop stop actions and an observed stopping state suppress
that alert. An external stop completed between probes can still appear unexpected.

Local diagnostics live under `<userData>/diagnostics`, accessible through **Open Data Directory**.
Desktop and solver diagnostics each retain five files of up to 1 MiB, with details limited to
16,384 characters and files created mode `0600` on Unix. The main process and its solver child
handles share one desktop logger, so solver exits do not create extra one-line files.
The shared secret scrubber runs before each write. Local logging works with remote reporting
disabled and does not upload files or memory dumps.

The solver loads `solver-diagnostics.js` in development or `desktop/solver-diagnostics.mjs`
in packaged builds through Node's `--import`. It records handled console errors and observes
fatal exceptions and unhandled rejections without suppressing process failure. Writes are
synchronous; logging failures do not stop filling. Solver stdio stays detached from Electron.
Native aborts and OS kills require OS crash records.

Renderer-exit notifications use the local log. Existing Sentry child-process reporting handles
remote crash events; recovery errors still use desktop error reporting. Build inputs, opt-out
and scrubbing are documented in [Sentry error reporting](2026-10-14-simplex-sentry-error-reporting.md).
