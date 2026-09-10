# 2026-09-08 — The pre-auth deadline lives on the stream, not on the ssh2 connection

ssh2 raises its connection event from `onHeader`, after a complete SSH identification line. A
peer that sends a partial line reaches no handler in `EmbeddedSshServer` at all, so the 30s auth
timer never armed and the stream was held for as long as the peer liked — invisible, because
`live` is also incremented there. The timer therefore moved into `inject()`, which sees every
stream.

Cancelling it needs the reverse link, connection → stream, and ssh2 gives the connection handler
only `(conn, info)`. The options were to parse packets, to wrap every stream in a proxy (which
does not help — the correlation problem is identical), or to read `conn._sock`, which ssh2 sets
in its Connection constructor. We read `_sock`, with a guard: the first time the lookup misses,
every armed deadline is disarmed and an error is logged. Failing to reap connections is a leak;
reaping the wrong stream would cut a live operator session. Two tests drive the paths that
depend on the correlation, so an ssh2 upgrade that renames the field fails loudly.
