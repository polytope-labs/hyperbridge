# Simplex desktop security model (#1242)

The Electron shell restricts its sandboxed renderer to `simplex://local`, keeps
Node, preload, IPC, webviews, production DevTools, external navigation, and
unapproved browser permissions unavailable, and launches Simplex only over its
private socket or named pipe. First-run config still uses the CLI's atomic
owner-only writer. The lasting boundary and CSP rationale are recorded in
[the desktop-renderer decision](../decisions/2026-09-15-the-desktop-renderer-has-no-electron-bridge.md).
