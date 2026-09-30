# 2026-09-03 — Show configured market prices in setup summaries

Updated setup-wizard market rows to show the first configured Buy and Sell prices, including their
`token1/token0` unit, as soon as either curve has a value. The order-cap summary was removed from the
row while the cap input remains available in the Configure editor.

Files: `ui/src/wizard/steps/Strategies.tsx`, `ui/src/styles/{markets,responsive}.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
