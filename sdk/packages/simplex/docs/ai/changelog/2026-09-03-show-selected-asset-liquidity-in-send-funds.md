# 2026-09-03 — Show selected-asset liquidity in Send funds

The Send funds form now displays the selected chain and asset's available balance beside the Amount
label. ERC-20 values reuse the canonical wallet-reserve and vault-aware balance snapshot, native gas
uses the chain's native balance, unavailable reads remain explicit, and a successful transfer triggers
an immediate dashboard refresh.

Files: `ui/src/operator/{Operator,Operations}.tsx`, `ui/src/styles/operator.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
