# 2026-09-16 — One bid per limit order

The matcher returned exactly one order, to keep the draw-down a one-to-one piece of bookkeeping. That
convenience cost fills: when the best-priced order's depth did not cover the swap, the filler skipped
it cross-chain or under-filled it same-chain while holding inventory a level down that would have
cleared the rest.

`matchLimitOrders` now returns every order that can serve the swap, best price first: those whose
offer on the whole input clears what the swapper asked for, with something left to pay. Ordering is
deterministic to the last comparison, payout then what is left then id. `matchLimitOrder` stays as
the single-best wrapper for callers that only need to know whether anything matches.

## Each order bids for itself

Simplex sends one bid per matching order, each carrying that order's own figure:

```
payout_i = min(offerFor(inputNet, price_i), available_i)
```

Three matching orders means three bids, not one bid for their sum and not one for a blended rate.
The full input is priced against each order independently, which is right precisely because each bid
is its own fill.

The gateway does the combining, and is built for it. Every fill clamps itself to what is outstanding
(`fillAmount = solverAmount > remaining ? remaining : solverAmount`), accumulates progress in
`_partialFills[commitment][outputToken]`, and clears `_filled[commitment]` on an under-fill so the
next bid can continue. The total delivered is therefore `min(Σ payout_i, T)` by construction, enforced
on chain rather than by arithmetic here. Summing offers in simplex billed one input to several orders
at once: two orders quoting 1500 and 1400 against 1000 USDC produced a combined 1,500,000 cNGN where
the orderbook had quoted the swapper 1,400,000.

Once an order is complete `_filled[commitment]` stays set and a later bid reverts with `Filled()`
(`IntentGatewayV2.sol`). That is an ordinary outcome, not an error: the bid gives its hold back and
the others are unaffected.

## What a bid holds

Each bid reserves against its own limit order alone. A hold that cannot be taken drops that bid and
leaves the rest of them alone, where the all-or-nothing `holdAll` used to drop the whole order. A bid
row still carries `reservations` as a list of `{ limitOrderId, amount }`, and `claimReservation`
hands it over exactly once.

The profit gate runs per bid against that bid's own payout, so a tail bid worth less than its gas is
refused on its own rather than hidden inside a combined figure. What the strategy reports for the
order is what its bids earn together.

Existing databases get a `reservations` column; the two columns it replaces are left behind rather
than migrated, since a hold outlives its bid by minutes and nothing reads a settled one.
