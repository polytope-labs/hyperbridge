# 2026-08-20 — A per-order cap can be removed, from the UI and over the API

The previous entry made `maxOrderSize` optional in config but left it one-way at runtime: an
operator could add a cap to an uncapped market, and could not take one off without editing the TOML
by hand. Both halves of that are now closed.

`AdminStrategy` gains `clearMaxOrderSize?: () => void` alongside `setMaxOrderSize`, implemented in
`adminStrategyFor` by setting the live `TradingPair.maxOrderSize` to `undefined` — the engine reads
the cap per order, so removal binds on the next evaluation exactly as a resize does.

`DELETE /api/strategies/:index/max-order-size` exposes it, on its own route rather than as a null on
the existing `PUT /api/strategies/:index`. `DELETE /api/strategies/:index` already means "remove the
market"; a cap removal one typo away from a market removal is not worth one fewer endpoint. The
handler is idempotent and refuses reference-only markets, matching the PUT.

`Simplex.clearMaxOrderSize(index)` is the library equivalent, shaped after the existing
`clearCurve`.

In the operator dashboard, the cap field is now blankable: emptying it turns the button into
"Remove cap" and issues the DELETE, while any other edit still PUTs. The button enables on any
change from the persisted value, including set-to-blank, which the old `!maxOrderSize.trim()` guard
disabled. The setup wizard's cap fields accept blank too and omit the key entirely rather than
emitting `""`, which config validation would reject as a malformed decimal rather than read as "no
cap".

Files: src/core/boot.ts, src/services/server/UiServer.ts, src/simplex.ts,
ui/src/operator/Operator.tsx, ui/src/wizard/state.ts, ui/src/wizard/steps/Strategies.tsx.
