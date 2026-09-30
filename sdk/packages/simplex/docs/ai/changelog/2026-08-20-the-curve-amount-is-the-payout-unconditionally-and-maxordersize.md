# 2026-08-20 — The curve amount is the payout unconditionally, and `maxOrderSize` is optional

Two changes to the same sizing decision.

**`targetOutput = policyMaxOutput`, capped or not.** The fix above still let the exposure cap clamp
the payout down to the user's pro-rata ask on a capped leg. It no longer does. `maxOrderSize` still
binds where it always did — `computeLegPolicyOutput` rations `token0ForLeg` against the pair's
remaining budget before the rate is applied, so a capped leg's curve amount is already the capped
slice's worth. What changes is the escrow side: paying above the pro-rata ask draws down more input
than the cap fraction nominally allots. That is more of the user's token for the same outlay, and
the outlay itself is still capped.

`capLimited` had to follow. It was `desiredOutput < output.amount` — true whenever a cap was active
at all — and it forces the partial-fill eligibility gate. With the payout no longer clamped to
`desiredOutput`, a curve running far enough above the order's rate can cover the whole ask out of a
capped slice; that is a full fill and gating it as a partial would reject cross-chain and calldata
orders the filler can actually serve. The condition is now `capFraction.lt(1) && policyMaxOutput <
output.amount` — the cap is active _and_ it actually shortens the fill.

**`maxOrderSize` is now optional.** `TradingPair.maxOrderSize` is `Decimal | undefined`; absent
means uncapped, and the pair fills every order at its full notional. `validatePairConfigs` no
longer requires it (a malformed value is still rejected), `tradingPairFrom` maps an absent TOML
value to `undefined` instead of the placeholder `new Decimal(0)`, and `sizeOrder` budgets an
uncapped pair against the order's own total so the per-pair ration still stops sibling legs
double-spending the same token0 without ever binding below the order. `FXFiller`'s constructor
check accepts absence and keeps exempting reference-only pairs, which never fill and whose callers
still pass a placeholder `0`.

Test updated: `validatePairConfigs > requires a positive maxOrderSize` asserted the removed throw;
it is now `accepts an omitted maxOrderSize, and rejects a malformed one`.

Files: src/strategies/fx.ts, src/config/pairs.ts, src/core/boot.ts, src/simplex.ts,
src/tests/pairs.test.ts.
