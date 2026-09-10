# 2026-09-01 — The balance series keeps its shape; #1159 §3 is left as a comment (#1159)

Not done, deliberately: the `trigger` enum, the nullable `transactionHash`, the
`{chain}-{token}-{solver}-{blockNumber}` id shape for event-triggered rows, and ordering "current liquidity" by
`snapshotTime` rather than `blockNumber`. All four change `LiquidityProviderBalanceV2`, which is live, and the
value is provenance metadata rather than correctness. Event-triggered rows therefore keep borrowing Hyperbridge's
head block, and the design is recorded as a comment on `recordProviderBalances` so a later migration has it to
hand. `SolverV4Positions` is additive — a new table, and no `@derivedFrom` field on `LiquidityProvider` either —
which is why it is in this change and §3 is not.
