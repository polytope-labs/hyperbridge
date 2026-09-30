# 2026-09-08 — Review fixes on the remote-access PR (#1217)

Six review comments from @ddboy19912, plus one bug found while re-running the tests.

- **The tunnel now connects only after the UI binds.** It used to start inside `startFiller`,
  so `--no-ui` or a lost race for port 8686 left it forwarding devices to whatever else
  answered on that port. `createTunnel` builds it (no network) and `startTunnel` connects it
  after `uiServer.start()` resolves; a bind failure stops and drops it.
- **Neither building nor starting the tunnel can stop filling.** Both are wrapped: a failure
  logs and leaves `tunnel` undefined.
- **Revoking a device now ends its live sessions.** `EmbeddedSshServer` tracks authenticated
  connections per fingerprint and `disconnectDevice` hangs up on them; `tcpip` also re-checks
  authorization per channel, so a revoked device opens nothing even before the hang-up lands.
- **Password and keyboard-interactive attempts count as failed logins.** They previously
  bypassed `fail()` entirely. The `none` method is excluded — it is the probe every client
  opens with, and counting it would have rate-limited legitimate devices out.
- **The failed-login table is swept and capped** (60s sweep, 10k sources, oldest-touched
  evicted), so one-off scanner addresses cannot accumulate.
- **`known_relay` holds one line per relay address.** A single line meant moving from relay A
  to B and back left A with no pin, trusting its next key blind.
- **Generated key pairs are parsed before use.** ssh2's ed25519 generator emits a pair its own
  parser refuses roughly once in 256 (measured 0.4–0.6% over 5,000 pairs — a dropped leading
  zero byte). For `operator_key` and `host_key` that is permanent breakage, not a transient
  error, since the bad key is written to disk and reloaded every boot. `generateKeyPair`
  retries up to 8 times. This was showing up as a ~1-in-3 flake across full runs of
  tunnel.test.ts.

Files: src/bin/simplex.ts, src/services/tunnel/EmbeddedSshServer.ts,
src/services/tunnel/TunnelService.ts, src/services/tunnel/keys.ts, src/tests/tunnel.test.ts.
