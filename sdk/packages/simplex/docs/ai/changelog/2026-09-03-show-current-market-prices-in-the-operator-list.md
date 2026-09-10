# 2026-09-03 — Show current market prices in the operator list

Operator market rows now show the first valid configured buy and sell prices with their price unit.
Missing sides, venue-priced markets, and reference-only entries remain free of fabricated values; the
underlying market data and pricing behavior are unchanged.

Files: `ui/src/operator/OperatorMarkets.tsx`, `ui/src/styles/{operator,responsive}.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
