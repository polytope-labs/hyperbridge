# 2026-09-03 — Recompose operator market drawers

Reworked the shared drawer spacing and live-market editor around the onboarding UI's open editorial
hierarchy. Market identity, risk limits, pricing directions, and actions now flow as flat sections
separated by restrained rules instead of nested cards; only the price chart retains a quiet visual
canvas. Order-limit controls align on one baseline, each direction uses the full width, previews and
point inputs share a balanced desktop row, and disabled sides use an inline action. Create-market and
standard operator drawers inherit the same header spacing and overflow-safe shell.

Files: `ui/src/operator/markets/StrategyMarketEditor.tsx`,
`ui/src/styles/{operator,responsive}.css`, and `docs/ai/{ChangeLog,Decisions,Flow}.md`.
