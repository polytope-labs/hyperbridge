# 2026-09-21 — cNGN inputs match, and outbid bids are retracted

## Symbols match case-insensitively

`matchLimitOrders` compared a limit order's input leg, spelled as the orderbook lists the book
(`cNGN`), with the incoming order's input symbol from the asset registry, which upper-cases every
symbol (`CNGN`). Every swap paying cNGN in matched nothing and was skipped with
`No limit order matches this order`. `serves()` and the same-asset check in `FXFiller` now compare
through `normalizeSymbol`.

## A rival's completed fill retracts our bids

`EventMonitor.handleFill` stopped at `if (!ours) return`, so a solver outbid on an order never
retracted its bids. Their limit-order holds and 0.1 BRIDGE deposits stayed locked until the
stale-bid sweep, which only takes bids older than an hour.

`ScannedFill` now carries `complete`: `true` for `OrderFilled`, which the gateway emits only once
every leg is filled, and `false` for `PartialFill`. It is optional, and a scanner that leaves it unset
is treated as not complete. `orderFillObserved` passes it through. When another solver completes an
order this filler holds an unretracted bid on, `IntentFiller.handleRivalCompletion` retracts the
bids at once. The retraction returns the deposit and releases the holds. A rival's partial fill
leaves the bid standing, since it may still fill the rest.
