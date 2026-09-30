# 2026-09-03 — Improve dashboard token balance cards

Reworked each network's token balances into image-led cards using the existing Simplex token asset
library. The new hierarchy separates total ownership from wallet and vault balances, highlights the
amount currently available to fill, and keeps partial or unavailable data visibly distinct without
presenting it as zero. Network gas remains visible in a compact network header.

Files: `ui/src/operator/OperatorOverview.tsx`, `ui/src/styles/{operator,responsive}.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
