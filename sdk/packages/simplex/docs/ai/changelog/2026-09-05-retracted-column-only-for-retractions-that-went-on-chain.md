# 2026-09-05 — Retracted column only for retractions that went on chain

A bid the pallet no longer holds (`BidNotFound`: our fill consumed it, or it never landed) is
marked retracted with a null extrinsic hash; the Retracted column showed a bare time for it. It now
shows a dash unless `retractExtrinsicHash` is set, so every entry in that column links to a real
retraction on Statescan.
Files: `ui/src/operator/Orders.tsx`, `docs/ai/{ChangeLog,Flow}.md`.
