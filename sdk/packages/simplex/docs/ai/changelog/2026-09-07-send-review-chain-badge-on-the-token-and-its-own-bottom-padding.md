# 2026-09-07 — Send review: chain badge on the token, and its own bottom padding

`.market-dialog-body` pads horizontally only, so the review's buttons sat flush against the dialog's
bottom edge; the review now supplies its own vertical padding rather than changing the shared body,
which every other dialog already compensates for in its own way. The network logo moved from a line
of its own onto the token icon as a bottom-right badge, the way wallets show it, leaving the network
as plain text beside the amount.

Files: `ui/src/components/SendConfirmDialog.tsx`, `ui/src/styles/operator.css`, `docs/ai/ChangeLog.md`.
