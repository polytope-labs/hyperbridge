# 2026-08-20 — Fills pay the curve amount again, not the user's requested amount

`FXFiller` had stopped overfilling entirely: every fill paid out exactly
`order.output.assets[i].amount`. `targetOutput` was `min(policyMaxOutput, desiredOutput)`, and
`desiredOutput` is `output.amount` whenever the leg is not exposure-capped. Combined with the
acceptance gate just below it (`if (policyMaxOutput < desiredOutput) return 0` — skip when the
curve pays less than asked), the two form a pincer: any order that survives to a fill has
`policyMaxOutput >= desiredOutput`, so `targetOutput` was always `desiredOutput`. Not an edge
case — 100% of non-balance-limited fills paid the requested amount and nothing more, whatever the
configured exchange rate said. Observed on Base fill `0x60b299e8...`: 25.469 ycNGN redeemed to
1,380 cNGN and exactly 1,380 cNGN forwarded, no surplus transfer and no `DustCollected`.

Introduced by #1123, which added `capFraction`/`desiredOutput` for the `maxOrderSize` exposure cap
and then reused `desiredOutput` as the general fill target — conflating "the slice the cap allows"
with "the amount to pay". Before #1123 the target was `policyMaxOutput` outright. No test asserts
the payout against the curve, so it landed silently.

`targetOutput` is now `policyMaxOutput` in every case — see the entry below, which took the cap
branch back out after this landed.

Files: src/strategies/fx.ts.
