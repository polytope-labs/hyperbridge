# 2026-08-20 — The filler only takes single-leg orders

`EventMonitor.handleOrder` now forwards an order only when it has exactly one input asset and one
output asset. Anything else is dropped at the door with an `orderSkipped` event carrying the reason
`Multi-leg order`, so the operator sees it in the activity feed rather than losing it silently.

The check runs before the de-duplication set is touched: a rejected order id is never marked as
seen, so a later single-leg order sharing that id is still delivered.

Downstream multi-leg handling is untouched — `FXFiller`'s per-leg loops and the leg-splitting in
`filler.ts` still run for phantom orders, which do not come through this path.

Files: `src/core/event-monitor.ts`, `src/tests/core/chain-lifecycle.test.ts`, `package.json`.
