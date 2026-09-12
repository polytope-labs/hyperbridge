# 2026-09-07 — 0.15.0: remote access from a device through the simplex-tunnel relay

New `[simplex.tunnel]` feature. `TunnelService` keeps an outbound `ssh2` session to the relay
(default `simplex.tunnel.polytope.technology:443`), requests a remote forward, and injects every
connection the relay hands back into `EmbeddedSshServer`: public-key auth against the paired
devices in `<data-dir>/tunnel/authorized_keys`, `direct-tcpip` only to the UI bind, everything
else refused, per-connection and per-source failure limits, 30s pre-auth timeout. The relay host
key is pinned from `relayHostKey` or on first contact (`tunnel/known_relay`). Reconnects with
backoff; never runs in init mode; a tunnel failure never touches filling.

UI: `Operations > Remote access` sheet (`ui/src/operator/RemoteAccess.tsx`): status, enable
toggle, relay address, device list with revoke, pairing that shows the private key once with a
QR code and the connection details. Routes: `GET/PUT /api/tunnel`, `POST /api/tunnel/devices`,
`POST /api/tunnel/devices/revoke`; `ConfigDto.tunnel` summary. Config emission for the block.
`ssh2` added as a dependency and to tsup's external list (it probes for an optional native
binding); it is CommonJS, so the code uses default imports — named imports only work under
vitest. `qrcode` added for the UI. Not added to the CLI wizard by request.

Verified: 13 new tests (`tunnel.test.ts` with an in-process fake relay, `ui-server-tunnel.test.ts`),
plus a smoke test against the real Rust relay with a stock OpenSSH client as the phone: page
served through `-L`, host key pinned matched, shell/other-port/stranger-key refused.

Files: `src/services/tunnel/{TunnelService,EmbeddedSshServer,keys,index}.ts`,
`src/services/server/{UiServer,dto}.ts`, `src/bin/simplex.ts`, `src/config/filler-toml.ts`,
`src/cli/init/emit-toml.ts`, `tsup.config.ts`, `package.json`, `filler-config-example.toml`,
`README.md`, `ui/src/operator/{RemoteAccess,Operations}.tsx`, `ui/src/types.ts`,
`ui/src/styles/operator.css`, `src/tests/{tunnel,ui-server-tunnel}.test.ts`, `sdk/pnpm-workspace.yaml`
(ssh2/cpu-features build scripts declined), `sdk/pnpm-lock.yaml`.
