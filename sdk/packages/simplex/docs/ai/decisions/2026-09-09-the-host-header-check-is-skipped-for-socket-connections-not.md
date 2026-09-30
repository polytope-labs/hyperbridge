# 2026-09-09 — The host-header check is skipped for socket connections, not relaxed

Decided: `hostHeaderAllowed` is not consulted at all when `provenanceOf(req.socket) === "unix"`. Its
signature and its logic are untouched, and the TCP path through it is byte-for-byte what it was — it is
still the DNS-rebinding defense, and the existing rebinding test still passes unchanged.

Why skip rather than widen the allowed set: the check exists because an attacker page can resolve its
own domain to a loopback address and become same-origin with the dashboard. Nothing about that applies
to a Unix socket. A browser cannot open a socket file at all, so there is no origin to rebind and
nothing for the check to defend; and an HTTP client over a socket puts whatever it likes in `Host`,
since there is no authority to derive one from. Measured on Node 24: `node:http` with `socketPath`
sends `Host: localhost`, which the existing rule happens to accept — so the breakage is not universal,
but it is arbitrary. A client built on any other base URL (`http://simplex/`, `http://unix/`) is a 403
for a reason that protects nothing, and that is a trap for the embedding app rather than a defense.

Rejected: adding `unix` to the accepted host set, or accepting any Host when the server is
socket-bound. Both leave a rule running that cannot fail usefully and can still fail wrongly — the
next client with an unusual base URL hits it again. If a check defends nothing on a transport, not
running it is clearer than tuning it forever.

Rejected: keying the skip off `this.boundLoopback` or the listen mode instead of the socket's
provenance. A tunnelled connection arriving at a socket-only server would then also skip the check,
which is a change to the tunnel's rules made by accident. `boundLoopback` stays `true` in socket mode
precisely so tunnelled connections keep the behaviour they had.
