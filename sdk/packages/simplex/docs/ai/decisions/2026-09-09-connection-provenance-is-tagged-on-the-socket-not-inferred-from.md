# 2026-09-09 — Connection provenance is tagged on the socket, not inferred from the listener

Decided: every socket carries a `PROVENANCE` symbol holding `tcp`, `unix` or `tunnel`. The listener
stamps what it accepted (prepended to `connection`, so the stamp lands before http attaches a parser),
`accept()` stamps what the tunnel injected, and `markProvenance` keeps the first stamp so an injected
channel is never relabelled by the listener's blanket one. `handle` reads `provenanceOf(req.socket)`.

This generalises `VIA_TUNNEL`, which answered one boolean question. A Unix socket makes it three, and
the request rules genuinely differ per provenance: a socket reaches only the owning user, a tunnel
connection is an authenticated device with *fewer* rights than a local caller (it cannot manage remote
access), and a TCP port reaches every local user and every web page on the machine.

Why a symbol set on our own socket, rather than a header or a flag: it is unforgeable by construction.
The value is set on an object this process created, and nothing a client can put on the wire reaches
it. That was already the reason `VIA_TUNNEL` was a symbol; widening the value does not weaken it.

Rejected: branching on what the server is listening on (`this.listenProvenance` read at request time,
with no per-socket tag). It is one field instead of a stamp, and it is right today because each server
listens one way. But tunnel connections are *injected* and never listened for, so the listening mode is
already not the truth for them — the tunnel would have to keep its own marker anyway, leaving two
mechanisms answering the same question. Tagging also makes `provenanceOf` total, so a socket arriving
by some future path with no stamp is a visible bug rather than one that silently inherits whatever the
listener happens to be.

Rejected: keeping `VIA_TUNNEL` alongside a new socket flag. Two symbols, two call sites to keep in
step, and the pair can disagree; `isTunnelled()` is now one line over the single tag and keeps its
signature, so no caller changed.
