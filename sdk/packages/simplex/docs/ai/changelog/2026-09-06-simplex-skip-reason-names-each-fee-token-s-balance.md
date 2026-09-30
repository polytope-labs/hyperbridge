# 2026-09-06 — Simplex skip reason names each fee token's balance

Selection logged a bare `simplex: insufficient stablecoin balance` when the solver held under one
whole token of every fee token, while the Circle branch already logged `circle: solver USDC balance
0 < 1000000`. Diagnosing a delegation that fell back to a native tx on Ethereum meant reading the
balances by hand. `buildSimplexPaymasterData` now returns the balances `selectToken` already read —
`{ insufficient: [{ symbol, balance, required }] }` in selection order — instead of `null`, and
`buildPaymasterAndData` records them as `simplex: solver USDC balance 0 < 1000000, USDT balance
0 < 1000000` (or `simplex: no fee token configured` when the chain lists neither). The builder and
`resolvePendingPermit2Approval` share a new `configuredFeeTokens` helper, which is where the token
symbols now live.

Files: `src/services/paymaster/index.ts`, `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/types.ts`, `src/tests/services/{PaymasterSelection,SimplexPaymaster}.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
