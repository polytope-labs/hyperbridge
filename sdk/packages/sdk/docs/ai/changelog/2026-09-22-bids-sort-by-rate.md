# 2026-09-22 — Bids sort by rate

`BidManager.sortBids` (and `IntentGateway.sortBids`, which delegates to it) is now a single sort
by rate, best to worst. It used to pick one of four strategies from the order's output tokens: an
exact rate for one output, a $1-per-stable sum for all-stable outputs, DEX-quoted USD values for
mixed outputs, and a raw-amount sum when pricing failed.

A leg's rate is the bid's output over its input on that leg. Every leg of an order trades the same
pair (#1311), so a bid's rate is its total output over its total input across the legs it quotes.
Rates are compared exactly by cross-multiplication, and equal rates keep arrival order. Sorting no
longer calls a DEX.

A bid is dropped when it:

- doesn't quote the order's legs one for one;
- names another token on a leg;
- quotes an input without an output, or the reverse;
- quotes no leg;
- quotes a leg below the order's rate (`output · escrow < input · required`).

`Bid.outputUsdValue` still prices outputs through DEX quotes; only ranking stopped using it.
