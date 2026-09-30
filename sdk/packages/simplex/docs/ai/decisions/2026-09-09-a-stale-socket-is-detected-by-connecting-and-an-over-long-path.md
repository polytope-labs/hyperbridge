# 2026-09-09 — A stale socket is detected by connecting, and an over-long path is an error

Decided: before binding a socket path that exists, dial it. `ECONNREFUSED` means the file outlived its
listener, so unlink and rebind. Anything that answers is a live instance and the start fails. `ENOENT`
means it vanished under us; proceed. Any other error (`EACCES` on somebody else's socket) fails with a
message saying we could not tell, because deleting it would be guessing.

Why not an existence test: a live socket has a file too, so existence cannot distinguish the two and
the check would either be useless or would steal a running instance's path. Verified against a real
`SIGKILL`ed process: the file survives, connecting to it gives `ECONNREFUSED`, and binding over it
gives `EADDRINUSE` — which `start()`'s `once("error", reject)` turns into a rejected start.

Refusing a live socket is deliberate and not merely defensive: the socket file is also the desktop
app's discovery mechanism and single-instance lock, so "someone is already serving here" is the answer
it needs, not something to overwrite.

Decided: a socket path over `sun_path` (103 bytes on macOS, 107 on Linux, NUL included) is a clear
error naming the size and the limit.

Superseded 2026-09-09 (same day): that error originally suggested `$TMPDIR` as the fallback. On Linux
`os.tmpdir()` is `/tmp`, mode 1777 — anything on the machine can create names there, and this path is
the daemon's address. It now points at `$XDG_RUNTIME_DIR` and says to avoid a shared directory.

Rejected: silently relocating to a short path under `$TMPDIR`. The caller uses this path to find the
daemon again — it is the discovery mechanism — so moving it trades a legible startup error for a
daemon nothing can attach to, which is strictly worse. The caller can implement that fallback itself;
it cannot recover from a socket at an address it was never told about. Without the guard the failure is
`listen EINVAL: invalid argument <path>`, which names neither the limit nor the reason.
