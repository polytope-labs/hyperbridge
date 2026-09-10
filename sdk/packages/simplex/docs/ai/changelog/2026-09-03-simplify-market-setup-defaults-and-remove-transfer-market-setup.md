# 2026-09-03 — Simplify market setup defaults and remove transfer-market setup

Removed the setup wizard's dedicated same-token transfer section and its prefab, state, prefill, and
stylesheet plumbing. Setup and operator market creation now use the normal cross-asset editor, reject
same-asset creation, default new order caps to `50000`, and prefill newly added curve-point sizes with
`1`. Optional field labels now use brackets for clarity.

Files: `ui/src/{components/CurveEditor,operator/markets/{CreateMarketForm,StrategyMarketEditor,useCreateMarket},wizard/state,wizard/steps/Strategies,wizard/strategies/{MarketRow,UniswapPositionsDialog,useStrategiesModel}}`,
`ui/src/styles/{markets,responsive}.css`, `src/cli/init/steps/strategies.ts`,
`src/services/server/{dto,setup-api}.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
