# 2026-09-14 — The Electron shell does not own the solver process

Decided: the desktop app is an attach-or-launch client for one detached Simplex process per Electron
user-data directory. It derives a stable Unix socket or Windows named pipe from that directory,
probes `/health`, and either attaches to a recognized Simplex listener or launches the existing
`dist/bin/simplex.js` with the bundled Node runtime. Closing or crashing Electron does not stop the
solver.

The process boundary is deliberate. Simplex can hold funded accounts and in-flight fills, so a UI
close is not a stop instruction. The operator's explicit Stop action remains the graceful shutdown
path; Electron never calls it as part of window or app teardown. A later launch finds the same local
address and attaches without spawning a second solver. Continuous crash supervision, tray lifecycle,
login startup, and focus restoration remain separate lifecycle work.

The shell never searches `PATH` and never rebuilds or copies the SPA. In development it resolves a
verified staged Node executable plus the existing Simplex binary and UI artifacts; a future packaged
layout must provide the same three resources explicitly. Missing resources are fatal before a window
is created.

The local transport is also the security boundary. On Unix, Simplex creates the socket mode `0600`.
Electron exposes it to its sandboxed renderer only through the privileged `simplex://local` scheme,
with no preload, IPC API, Node integration, or TCP listener. A live endpoint whose `/health` response
is not Simplex is never overwritten. Windows named-pipe identity and authentication hardening remain
tracked separately; this decision does not claim that a pipe name alone authenticates its peer.

The protocol handler preserves request methods, paths, queries, headers, streaming bodies, response
status, response headers, streaming output, and cancellation. Transport failures reject instead of
becoming synthetic HTTP responses because a terminal HTTP status permanently closes `EventSource`,
while a failed fetch lets Chromium reconnect after a solver restart.

Rejected alternatives:

- Bundling another UI would create two renderer release artifacts that can drift.
- Using a random socket or port would prevent deterministic reattachment and expose a TCP surface.
- Making the daemon Electron's child lifetime would turn a window close or renderer crash into an
  operational stop.
- Treating every existing socket as stale would allow the desktop app to delete or replace an
  unrelated live listener.
- Translating socket failures to HTTP 503 would strand the Orders activity stream after restart.

The executable contract is covered at three levels: unit tests pin path selection, spawn flags,
health decisions, timeout behavior, staging verification, and proxy fidelity; a real Electron test
hard-kills and relaunches the shell, restarts a real `UiServer`, reloads the renderer twenty times,
and writes the first-run config; CI repeats that test on macOS arm64, Linux x64/arm64, and Windows
x64 after staging the host's signed Node 24.19.0 runtime.
