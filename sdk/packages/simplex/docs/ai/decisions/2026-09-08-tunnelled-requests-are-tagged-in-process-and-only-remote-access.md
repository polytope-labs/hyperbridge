# 2026-09-08 — Tunnelled requests are tagged in-process, and only remote-access routes are refused

Two ways to tell a tunnelled request apart were on the table. Dialling the UI from a distinct
loopback source (`localAddress: "127.0.0.2"`) needs no plumbing, but only Linux hands out
127.0.0.0/8 freely — it would silently fail to tag on macOS, and a guard that silently stops
guarding is worse than none. So the channel is handed to the HTTP server directly with
`server.emit("connection", channel)` and a symbol on the socket. The cost is a few no-op socket
methods the HTTP server calls (`setTimeout`, `setNoDelay`, `setKeepAlive`, `ref`, `unref`,
`destroySoon`); the gain is a tag that cannot be forged and one less TCP hop.

What to refuse was Seun's call, and the answer was `/api/tunnel*` only. The alternative was to
also block the money and lifecycle routes (`/api/send`, `/api/vault/*`, `/api/stop`,
`/api/config`, `/api/log-level`), which would make a lost phone harmless — but remote access
exists so an operator can run the filler from their phone, and the UI and docs already say
plainly that a paired key opens the whole dashboard. The specific defect was narrower than "the
phone is powerful": pairing a second key over the tunnel outlives revoking the first, so
revocation did not mean what it says. Reads stay allowed so the panel can render itself
read-only, which is friendlier than a button that 403s.
