# 2026-09-09 — A socket path is type-checked with lstat before anything is probed or removed

Decided: `clearStaleSocket` calls `lstatSync` first and refuses anything that is not a socket. Only a
socket is ever a candidate for the connect probe or for `unlink`.

Why the probe cannot come first: the invariant the previous version rested on — "ECONNREFUSED means the
file outlived its listener and is a corpse" — is false. connect(2) answers ECONNREFUSED for a regular
file, a FIFO and a directory exactly as it does for an orphaned socket. So a probe-only test classified
an operator's file as stale and deleted it; `--ui-socket ~/filler-config.toml` destroyed the config,
reproduced against a file holding a signer key. Only a live socket is distinguishable by probing, which
is the one thing `lstat` cannot tell us — so each check does the part the other cannot.

Why `lstat` and not `existsSync`/`stat`: `existsSync` follows symlinks, so a dangling one read as
absent, nothing was cleaned up, and the bind then failed with a bare `EADDRINUSE` naming no cause — a
permanent, unrecoverable start failure that the stale-socket recovery was specifically meant to prevent.
Not following the link also means a symlink planted at the path can never redirect a later operation.

Rejected: unlinking a non-socket after warning. The path is operator-supplied and typo-prone, and the
value of what might be there (a config with a signing key) is far higher than the convenience of
auto-clearing it. Refusing costs one manual `rm` in the rare legitimate case.
