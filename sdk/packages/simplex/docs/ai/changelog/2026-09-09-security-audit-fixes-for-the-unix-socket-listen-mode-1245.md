# 2026-09-09 — Security audit fixes for the Unix socket listen mode (#1245)

A multi-agent security audit of the socket listen mode (8 independent lenses, findings deduped and put
through 3-lens adversarial refutation) produced two surviving findings and one ordering bug; all three
are fixed here, each pinned by a test that fails when the fix is reverted.

**The socket is now created `0600` instead of being chmod'ed to `0600` after binding.** Binding first
and narrowing after left the file at `0777 & ~umask` — 0775 under the common `umask 002` — for roughly
a millisecond. That is not a theoretical window: Linux checks the mode at connect(2) and *never
re-checks*, so a local user who connects inside it keeps a fully privileged, unauthenticated session
for the life of the daemon — tightening the mode afterwards does not revoke an established connection.
The audit won that race in 7 of 8 attempts with a connect loop. `listenPrivate` now sets `umask 0177`
around the synchronous `listen` call, which makes libuv create the socket `0600` with no window at all;
verified directly (mode is `600` at bind, `775` without the guard). Because `process.umask` is
process-wide, it wraps only the synchronous call — libuv binds inside it, so no other JavaScript in the
process can run in between.

The old code's comment asserted this window "cannot be closed from Node". That was wrong, and it is
corrected rather than merely superseded, since the claim is what would have kept the bug unfixed.

`assertSocketIsPrivate` now **asserts** the mode rather than chmod-ing it. A repair there would restore
the mode only after the socket had been reachable at the wrong one — reintroducing the window it is
supposed to prove absent, and hiding the regression from the test. It also fails closed: the previous
version logged a warning and kept serving the fund-moving API on a group-writable socket.

**`clearStaleSocket` no longer deletes whatever sits at the socket path.** The stated invariant —
"ECONNREFUSED means the file outlived its listener" — is false: connect(2) answers ECONNREFUSED for a
regular file, a FIFO and a directory too, so `--ui-socket ~/filler-config.toml` silently deleted it
(reproduced against a file holding a signer key). The type is now checked with `lstat` before any probe
or unlink, and only a socket is ever a removal candidate. `lstat` rather than `existsSync` also fixes a
DoS: `existsSync` follows symlinks, so a dangling one read as absent, nothing was cleaned up, and the
bind then failed with a bare `EADDRINUSE` naming no cause.

**`listenProvenance` and `boundLoopback` are assigned inside the `listen` callback, not before the
bind.** Setting them first meant a rejected socket start on an already-listening server left a live TCP
listener with every connection tagged `unix` — and so exempt from the Host-header check, re-opening DNS
rebinding against `/api/send`. Reproduced against the real class. Not reachable from the CLI, which
picks exactly one transport, but `UiServer` is exported and the desktop app this feature exists for is
an embedder. `listenOnSocket` also refuses outright when the server is already listening.

Smaller items from the same audit: the over-long-path error no longer suggests `$TMPDIR` (on Linux that
is `/tmp`, mode 1777, and this path is the daemon's address) and points at `$XDG_RUNTIME_DIR` instead;
`--ui-socket ""` is rejected rather than falling through to the TCP port the operator was avoiding;
`isWindowsPipe` is gated on the platform, so a `\\.\pipe\` string on Linux is treated as the ordinary
file it is; and the `--ui-socket` help text no longer claims owner-only access unconditionally, which
this change's own Windows research contradicts.

Audit findings deliberately not acted on: path squatting (`--ui-socket` has no default, so there is no
well-known path to camp on, and the daemon is fail-closed on a live socket); chmod-through-symlink (the
lstat gate closes it, and the sticky bit forbids the precondition anyway); and TCP-to-socket bridging
(an operator who builds one has already converted a 0600 boundary back into an open port). Pre-existing
issues recorded, not fixed here: the server has no authentication in any mode, `serveStatic`'s traversal
guard uses a bare `startsWith`, and the stale `once("error")` handler swallows the first post-listen
server error.

Files: src/services/server/UiServer.ts, src/bin/simplex.ts, src/tests/ui-server-socket.test.ts,
docs/ai/{ChangeLog,Decisions,Flow}.md.
