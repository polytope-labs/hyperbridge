# 2026-09-07 — Make the remote-access switch move on click, not on the round trip

The switch was controlled by the server's `enabled` flag and only moved once `PUT /api/tunnel`
returned. The dashboard shares an event loop with the filler, so any request queues behind block
scanning and vault polling: measured on a running filler, even `/health` answered in 88-328ms with
outliers over a second, and `/api/tunnel` and `/api/status` sit in the same band. The switch now
moves optimistically (1ms measured) and reconciles when the response lands; a poll arriving
mid-flight can no longer flip it back. The badge follows the same optimistic value, and polling
tightens to 1s while the state is `connecting`/`reconnecting` so it settles quickly.

The underlying latency is process-wide and pre-existing, not specific to these routes.

Files: `ui/src/operator/RemoteAccess.tsx`, `docs/ai/ChangeLog.md`.
