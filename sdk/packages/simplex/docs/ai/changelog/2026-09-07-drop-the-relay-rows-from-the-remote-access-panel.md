# 2026-09-07 — Drop the relay rows from the Remote access panel

The relay address and its host key were shown as facts alongside an editable disclosure. Neither is
something an operator acts on: the hosted relay is the default, its key is pinned in the binary, and
a self-hosted relay is configured in `[simplex.tunnel]`. The panel now shows only what a phone needs
(public endpoint, host key to pin) plus open sessions; the `PUT /api/tunnel` relay field is untouched
for config and API use. Removed the `.tunnel-advanced` and `.tunnel-relay-row` styles with it.

Files: `ui/src/operator/RemoteAccess.tsx`, `ui/src/styles/{operator,responsive}.css`, `docs/ai/ChangeLog.md`.
