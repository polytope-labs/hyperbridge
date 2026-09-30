# 2026-09-08 — A paired device can no longer manage remote access

A second review pointed out that a tunnelled request is indistinguishable from one the operator
made at the keyboard: the embedded server dialled the UI on loopback, so `UiServer.handle` saw an
ordinary local request, and the only guards — the Host header and the constant `X-Simplex-UI`
header — are ones any client sets. A device holder could therefore `POST /api/tunnel/devices` and
pair a second key, which survives revoking the first: the revocation work from the last review
was defeated in one request. It could also repoint the relay at one it runs.

The tunnel now hands the SSH channel straight to the UI server in-process — `deliver` on
`EmbeddedSshServer`, `UiServer.accept`, and `server.emit("connection", channel)` — instead of
dialling 127.0.0.1. The channel carries a `VIA_TUNNEL` symbol that nothing off the wire can
forge, so `handle` refuses any non-GET `/api/tunnel*` from a device, and `GET /api/tunnel`
reports `readOnly: true` so the panel renders itself read-only rather than failing on the first
click. This also removes a loopback TCP hop, and the device's real origin now reaches the HTTP
layer as `req.socket.remoteAddress`.

Everything else a device can do is unchanged — Send, the treasury tools, pause, curve edits —
which is the documented model and Seun's call: the fix is that revoking a lost device now
actually takes everything away.

Files: src/services/tunnel/EmbeddedSshServer.ts, src/services/tunnel/TunnelService.ts,
src/services/server/{UiServer,http-util,dto}.ts, src/bin/simplex.ts,
ui/src/operator/RemoteAccess.tsx, src/tests/tunnel.test.ts (25 tests).
