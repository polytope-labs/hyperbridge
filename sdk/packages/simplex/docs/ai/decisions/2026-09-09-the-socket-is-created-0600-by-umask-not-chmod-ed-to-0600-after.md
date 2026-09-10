# 2026-09-09 — The socket is created 0600 by umask, not chmod'ed to 0600 after binding

Decided: `listenPrivate` sets `umask 0177` around the synchronous `listen` call so libuv creates the
socket file `0600`. The mode is never briefly wider.

Why not bind then chmod, which is the obvious shape and what this change originally did: AF_UNIX
permissions are checked at connect(2) and never re-checked, so the window is not "a moment of exposure"
— a connection opened inside it is served for the life of the daemon, and tightening the mode does not
revoke it. Measured at roughly a millisecond, and won 7 times out of 8 by a connect loop. On an
unauthenticated API where `/api/send` moves funds and init mode holds private keys, that is a real
local privilege boundary, not a hardening nicety.

Rejected: documenting the window and telling embedders to use a private parent directory. That was the
original decision here and it was wrong on the facts — it rested on the claim that the window "cannot be
closed from Node", which is simply false. A private parent directory is still good advice, but it is
defence in depth, not the fix, and stating it as the fix is what kept the bug in place.

Rejected: bind to a temporary name in the same directory, chmod it, then rename over the real path.
Also race-free and it avoids touching a process-wide setting. But libuv records the bound name and
unlinks *that* on close, so after a rename the real socket file would survive every clean shutdown —
trading a security bug for a litter bug that the stale-socket path would then have to clean up.

On `process.umask` being process-wide: it wraps only the synchronous `listen` call. libuv binds inside
that call, so no other JavaScript in this process can run between the set and the restore. It throws on
a worker thread, hence the guard; the UI server runs on the main thread.

Consequence for `assertSocketIsPrivate`: it asserts the resulting mode and does NOT chmod. A repair
there would restore the mode only after the socket had been reachable at the wrong one — recreating the
exact window, while also hiding the regression from the test that is supposed to catch it. A wrong mode
now refuses to serve (a default ACL on the containing directory is the likely cause, and the error says
so). Failing closed is the same rule applied to a chmod failure, which an earlier version downgraded to
a warning while continuing to serve.
