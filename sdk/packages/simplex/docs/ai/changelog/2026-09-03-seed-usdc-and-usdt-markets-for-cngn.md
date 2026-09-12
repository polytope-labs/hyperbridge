# 2026-09-03 — Seed USDC and USDT markets for CNGN

The setup wizard now seeds both USDC/CNGN and USDT/CNGN markets when CNGN and USDT are available in
the selected network's token catalog. Networks without that catalog combination retain the existing
single-market default, and user-created or existing markets are unchanged.

Files: `ui/src/wizard/state.ts`, `ui/src/wizard/strategies/useStrategiesModel.ts`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
