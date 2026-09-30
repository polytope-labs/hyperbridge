# 2026-09-09 — `accept()` keeps its `listening` guard; the coupling is pinned by a test instead

Decided: `accept()` still returns false when `!this.server.listening`, unchanged. A test asserts a
socket-only server serves tunnelled connections, and that `accept()` refuses them when nothing is bound.

Tunnel connections are dialled outbound to the relay and injected with `server.emit("connection")`;
nothing listens for them, so the guard tests a condition the path does not logically need. It is
satisfied by a Unix listener today, so socket-only remote access works and there is no bug to fix —
which is exactly why it is worth a test: the trap springs later, when someone removes a listener that
appears unused and silently takes remote access with it.

Rejected: dropping the guard now. It is not dead — it is what stops a channel being handed to a server
that has been stopped, and `stop()` clears `_handle` synchronously so the check is meaningful. Removing
it as part of this change would trade a pinned, harmless coupling for an untested behaviour change in
the tunnel's hot path.

`EmbeddedSshServer`'s log line was reworded, though: it said "No UI to serve behind the tunnel" for
every refusal, which misdescribes the case where the UI exists and is merely unbound.
