# 2026-09-16 — Same-asset quotes return, with four smaller review fixes

## A limit order can take in and pay out the same symbol again

No orderbook book trades a symbol against itself, so the curve removal left no way to express the
same-asset case at all: `create` refused identical symbols and the engine could only have served one
from a row nothing could write. Those quotes are local now. A same-asset order is stored, prices
swaps here, and is never sent anywhere: it carries the symbol as both `base` and `quote`, comes back
from `create` as `unposted`, and `repost` and `reconcile` pass over it because it was never on a book.

It must be priced at or below par, since a fill hands back the same token it took in and anything
above par pays out more than it receives. That is the rule the curves expressed as ask-only and
priced under 1.

## An operator upgrading from curves is told their prices are dead

A `[[pairs]]` entry still carrying `bidPriceCurve`, `askPriceCurve`, `maxOrderSize` or `referenceOnly`
parses cleanly, because unknown keys are ignored rather than rejected. It also quotes nothing until a
limit order is posted. `bootFiller` now names those keys in a warning instead of leaving the operator
to work it out.

## An order nothing can value waits the deepest confirmation the curve allows

`getOrderUsdValue` answers null when no limit order gives the input symbol a route to a dollar, and
the stable-only base reads zero for an exotic. Sizing the confirmation wait on that zero took the
shallowest point of the curve for the order there was least reason to trust. Unknown now takes the
top of the curve, with a warning naming the order.

## Smaller

`expiresAt` is validated on the way in: one that cannot be parsed never arrives, and one already past
creates an order that is posted, paid for and never matched.

`toRaw`, `toScaled` and `offerFor` handle a token with more than 18 decimals. It is finer than the
orderbook's own unit, so the conversion goes the other way and there is nothing to quantise; the old
arithmetic raised `RangeError` on a negative exponent, which would have taken the pricing path down
rather than mispriced anything.
