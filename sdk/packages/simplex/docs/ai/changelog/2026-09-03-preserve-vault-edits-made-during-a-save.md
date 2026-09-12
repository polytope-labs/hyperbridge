# 2026-09-03 — Preserve vault edits made during a save

Vault saving now queues one latest-draft retry when the operator clicks Save again while an earlier
request is still in flight, so the shared action guard cannot silently discard newer values. The UI
also treats `persisted: false` as a save failure and renders the result inside the open vault drawer.

Files: `ui/src/operator/{Operations.tsx,Operations.test.tsx}` and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
