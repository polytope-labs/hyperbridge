# 2026-09-08 — Audit fixes on the embedded SSH server

An audit of the tunnel code (nine finder agents, then hand-verification with reproductions)
turned up eleven issues. No critical ones: nothing lets an unpaired key reach the dashboard.
Auth, the forward restriction, the CSRF header and the DNS-rebinding guard all held up, and the
claim that a revoked device keeps its live session was refuted by reproduction. What was wrong
was availability.

- **`--ui <non-loopback>` broke remote access completely.** The panel hard-coded
  `8686:127.0.0.1:<port>` as the forward while `isTarget` accepted only the bound address, so
  the device was told to open the one channel the server refuses. `connection()` now names the
  real target host.
- **A UI on a non-`.1` loopback address was unreachable.** `createTunnel` collapsed every
  loopback bind to 127.0.0.1 while `UiServer` bound exactly what it was given. Only wildcard
  binds collapse now.
- **Three teardown paths did not hang up.** `conn.end()` sends DISCONNECT and closes our write
  side only; a peer that ignores it kept the session, its protocol state and its slot in the
  connection count. The auth timeout, the failure limit and `disconnectDevice` now destroy the
  stream after a short grace.
- **A peer that never finished the SSH identification line was invisible and immortal.** ssh2
  raises its connection event at `onHeader`, so no per-connection timer ever armed. `inject()`
  now puts the deadline on the stream itself, before ssh2 sees it.
- **The client chose the key exchange.** ssh2 offers group16/17/18 by default and negotiates by
  the client's preference; group18 costs ~107ms of synchronous CPU per handshake, measured, on
  the loop that fills orders — and a rekey flood needs no login at all. The server now offers
  curve25519, the ECDH groups and group14.
- **Plain `zlib` was offered pre-authentication.** It starts compressing at NEWKEYS, before
  auth. Only `none` and `zlib@openssh.com` are offered now.
- **`relayHostKey` was not scoped to a relay.** Changing relays carried the old pin over and
  locked remote access out for good, with an error pointing at a file that path never reads.
  Both the runtime config and the persisted block drop it when the relay changes.
- **The built-in relay pin matched one exact spelling.** Writing the hosted relay without its
  port — which the config documents — skipped the shipped pin and fell back to trusting
  whatever answered. Pins are keyed by normalised `host:port` now.
- **The first-contact pin was written before the relay proved it held the key.** ssh2 calls
  `hostVerifier` during KEXDH_REPLY, before the signature over the exchange hash is checked.
  The pin is committed on `ready`.
- **The panel ignored `persisted`.** Turning remote access off against an unwritable config
  reported success and came back on restart — the lost-device path.
- **`inject()` assumed its stream was not a real socket.** `Object.assign` over getter-only
  `remoteAddress` throws; it uses `defineProperty` now, and timer-driven destroys are wrapped
  so a stream that refuses to close cannot take the process down.

Files: src/services/tunnel/EmbeddedSshServer.ts, src/services/tunnel/TunnelService.ts,
src/services/server/UiServer.ts, src/bin/simplex.ts, ui/src/operator/RemoteAccess.tsx,
src/tests/tunnel.test.ts (24 tests, 8 new).
