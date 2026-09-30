# 2026-09-03 — Make new web markets uncapped by default

New markets created from the operator form or setup wizard now leave the optional maximum-order field
blank, which emits no `maxOrderSize` and therefore creates an uncapped market.

Files: `ui/src/operator/markets/{CreateMarketForm.tsx,useCreateMarket.ts}`, `ui/src/wizard/state.ts`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
