# Remote access: from `enabled = true` to a phone loading the dashboard

Read from the source and exercised by `src/tests/tunnel.test.ts` and a smoke test against the real
relay on 2026-09-07.

1. `bin/simplex.ts` `startFiller` builds `TunnelService` after the filler boots (data dir, the
   `[simplex.tunnel]` block, and a lazy `uiTarget` that resolves the UI's bound port) and calls
   `start()`; it is a no-op unless `enabled`. The setup wizard path never reaches this until
   save-and-start, so init mode is never tunneled.
2. `TunnelService.connect` parses the relay address (default port 443), opens an `ssh2` client with
   the operator key from `tunnel/operator_key` (created on first use), keepalives every 15s, and a
   `hostVerifier` that compares the relay key's SHA256 fingerprint with `relayHostKey` or the
   `tunnel/known_relay` pin, storing it on first contact.
3. On `ready` it calls `forwardIn("0.0.0.0", 0)`; the relay answers with the leased port, which
   becomes `status().port`. The bind address is always `0.0.0.0` because `ssh2` matches incoming
   `forwarded-tcpip` channels on the exact `bindAddr:port` it asked for, and the relay echoes it.
4. Each `tcp connection` event is a phone: `accept()` yields a duplex channel, which gets
   `remoteAddress`/`remotePort` from the relay's originator fields and goes to
   `EmbeddedSshServer.inject` → `Server.injectSocket`. No local port is ever opened.
5. `EmbeddedSshServer.onConnection`: a 30s auth timer; `authentication` accepts only `publickey`,
   looks the fingerprint up in `authorized_keys` on every attempt (so revocation is immediate),
   verifies the signature, and after three failures ends the connection; a source with ten
   failures in ten minutes is refused at `inject`. After `ready`, `session`, `openssh.streamlocal`
   and global requests are rejected; a `tcpip` request is accepted only when its destination is
   the UI port on the UI host (or any loopback name when the UI is on loopback), then piped to a
   `net.connect` to the UI server.
6. The phone's browser sends `Host: localhost:8686`, which passes the UI server's loopback Host
   guard unchanged.
7. On `close` the client is dropped and `scheduleReconnect` retries with exponential backoff (1s
   doubling, capped at 60s), state `reconnecting`, until `stop()` or `configure({enabled:false})`.
8. `UiServer`: `GET /api/tunnel` returns `status()`; `PUT /api/tunnel` validates, rewrites
   `config.simplex.tunnel`, persists the file, then `configure()`s the live service;
   `status()` carries a `connection` block (host, port, username, host-key fingerprint, local
   forward) built by `TunnelService.connection()`, which pairing returns too, so the dashboard renders
   the SSH fields whether or not a device was just paired.
   `POST /api/tunnel/devices` with `publicKey` normalises and authorizes the phone's own key
   (`normalizePublicKey` → `TunnelKeyStore.addDevice`) and returns no private key; without it,
   `addDevice` mints a pair and returns the private key once; `POST /api/tunnel/devices/revoke`
   deletes the line. `RemoteAccess.tsx` polls the
   status every 3s while the sheet is open and renders the private key as a QR code with `qrcode`.
