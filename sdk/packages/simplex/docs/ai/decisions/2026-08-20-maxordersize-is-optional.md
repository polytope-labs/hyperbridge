# 2026-08-20 — `maxOrderSize` is optional

Chosen: `TradingPair.maxOrderSize` is `Decimal | undefined`. Absent means uncapped.

The cap was mandatory in `validatePairConfigs` but already optional in the TOML type, so
`tradingPairFrom` bridged the gap with `new Decimal(pair.maxOrderSize ?? "0")` — a placeholder that
only worked because reference-only pairs never reach the sizing path. A zero cap and no cap are
opposites, and encoding "no cap" as the most restrictive possible value is the kind of thing that
holds until someone routes a new pair type through the same code.

`sizeOrder` is where absence is resolved: an uncapped pair sets `cappedByPair` to the order's own
per-pair total and `capFraction` to 1. That keeps the per-pair ration in `computeLegPolicyOutput`
doing its other job — stopping two legs of the same pair from spending the same token0 twice —
without it ever binding below the order.

Alternatives rejected:

- _Keep it required and let operators write a very large number._ Works, but "uncapped" then has no
  representation, only an approximation, and the log line reads as a cap that happens not to bind.
- _Default an absent cap to `Infinity`._ Same behaviour, but `Decimal(Infinity)` propagates into
  `capFraction` division and into every log that stringifies the cap. `undefined` makes each
  consumer state what it does when there is no cap.

Kept deliberately: `assertPairValid` still exempts reference-only pairs from the positive-value
check. `FXFiller` takes `TradingPair[]` as a public constructor argument, and callers written
against the old required field still pass `new Decimal(0)` there. A reference pair never fills, so
its cap is never read either way — rejecting it would break those callers for nothing.

Since resolved: removal is now reachable at runtime — see "Removing a cap is its own endpoint".
