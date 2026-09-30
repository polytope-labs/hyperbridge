# 2026-09-05 — Order rows link only to HyperFX

Dropped the placement- and fill-transaction explorer links from each order row; the HyperFX order
page already shows both, and the two extra arrow icons crowded the row. The links cell keeps the
single HyperFX link. `explorerTxUrl` and the per-row fill lookup are gone with them.
Files: `ui/src/operator/Orders.tsx`, `docs/ai/{ChangeLog,Flow}.md`.
