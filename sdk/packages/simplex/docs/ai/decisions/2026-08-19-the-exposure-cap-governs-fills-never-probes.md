# 2026-08-19 — The exposure cap governs fills, never probes

Chosen: `computeLegPolicyOutput` takes `remainingToken0: Decimal | null`, and `quotePhantomFill`
passes `null`. The pair's `maxOrderSize` budget rations real fills only.

The two paths are not the same kind of number. On a fill the output is an amount to pay out, and
`maxOrderSize` is a real exposure limit — clamping is the point. On a probe the output is a
_price_: the leg is quoted against a fixed standard amount and every consumer recovers the rate as
`medianPrice / standardAmount`. Clamping the quantity there does not reduce exposure (there is
none), it just makes the numerator smaller than the denominator assumes, and the published rate is
wrong by exactly the clamp ratio — silently, with no error and no warning.

Alternatives rejected:

- _Skip the leg when the probe exceeds the pair's cap._ Honest, but it removes the pair from the
  price feed entirely and zeroes its depth downstream. A correct price for a size the filler
  would cap is more useful than no price, and the warning covers the operator's need to know.
- _Clamp, then scale the output back up._ Identical to not clamping for a linear curve, and
  actively misleading on a sloped one, since the scaled figure would not be a price the curve ever
  produced.
- _Leave it and raise `maxOrderSize` in operator config._ This is a code bug that produces a wrong
  published number; requiring every operator to know that would guarantee someone does not.
