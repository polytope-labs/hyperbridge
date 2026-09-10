# 2026-09-07 — Show the SSH connection fields in the Relay connection card

The card showed a public endpoint, a host key and a session count; only the first two were useful and
they were half of what an SSH app asks for. It now renders the whole connection — host, port,
username, host key fingerprint, local port forward — each copyable on its own, which is how Termius
and friends want them entered. `TunnelConnectionDto` is a new shared shape returned by both
`GET /api/tunnel` (`connection`) and pairing, built once in `TunnelService.connection()`. The
post-pairing card no longer repeats the fields, keeping the key material and the app instructions;
the open-session count moved next to the device count ("2 paired · 1 connected").

Files: `src/services/server/dto.ts`, `src/services/tunnel/TunnelService.ts`,
`ui/src/operator/RemoteAccess.tsx`, `ui/src/types.ts`, `src/tests/ui-server-tunnel.test.ts`,
`docs/ai/ChangeLog.md`, `docs/ai/Flow.md`.
