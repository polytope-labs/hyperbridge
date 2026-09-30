# 2026-09-07 — Pin the hosted relay's host key by default

`DEFAULT_TUNNEL_RELAY_HOST_KEY` (`SHA256:L6LT8Zu6Ke+k4cZLiDcUO/3EYWtH5vJXsVMPVnCy3ts`, read off the
deployed relay with `ssh-keyscan` and matched against the operator's record) is used whenever the
relay is the default and no `relayHostKey` is configured; `expectedRelayFingerprint` centralises
the precedence (config pin, built-in pin, first-contact pin). Verified end to end against the live
relay: OpenSSH operator got port 21047 and kept it across a reconnect, `TunnelService` connected
with the pin, an OpenSSH phone reached the local UI through `simplex.tunnel.polytope.technology`
with strict host-key checking, and a shell attempt was refused.

Files: `src/services/tunnel/{TunnelService,index}.ts`, `src/tests/tunnel.test.ts`, `README.md`,
`filler-config-example.toml`, `docs/ai/*`.
