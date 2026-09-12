# 2026-09-09 — Listen state is assigned after the bind succeeds, never before

Decided: `boundLoopback` and `listenProvenance` are set inside the `listen` callback in both
`listenOnPort` and `listenOnSocket`, and `listenOnSocket` refuses outright when the server is already
listening.

Why: `listenProvenance` decides whether the DNS-rebinding Host check runs at all. Assigned before a
fallible bind, a rejected socket start on an already-listening server left a live TCP listener with
every connection tagged `unix`, and therefore exempt from that check — full DNS rebinding re-opened
against `/api/send`, with `start()` having reported failure. Reproduced against the real class.

Not reachable from the shipped CLI, which selects exactly one transport and constructs one server per
address — but `UiServer` is an exported class and the desktop app this listen mode exists for is
precisely an embedder that might retry or attach-or-spawn. A security toggle that is fail-open by
statement ordering is worth fixing on reachability grounds alone, and the fix is to move two lines.

The callback runs before the event loop can deliver a connection, so there is no interval in which a
request is served under stale values.
