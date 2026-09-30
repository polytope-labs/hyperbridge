# 2026-09-16 — One bid per limit order

The matcher returned exactly one order, to keep the draw-down a one-to-one piece of bookkeeping. That
convenience cost fills: when the best-priced order's depth did not cover the swap, the filler skipped
it cross-chain or under-filled it same-chain while holding inventory a level down that would have
cleared the rest.

`matchLimitOrders` now returns every order that can serve the swap, best offer first: those whose
offer on the whole input clears what the swapper asked for, with something left to pay. Ordering is
deterministic to the last comparison: largest offer, then most left, then id. `matchLimitOrder` stays
as the single-best wrapper for callers that only need to know whether anything matches.

## Each order bids for itself

Simplex sends one bid per matching order, each carrying that order's own figure:

```
payout_i = min(offerFor(inputNet, price_i), available_i)
```

Three matching orders means three bids, not one bid for their sum and not one for a blended rate.
The full input is priced against each order independently, which is right precisely because each bid
is its own fill.

An order carrying output calldata is the exception: it takes exactly one bid. The attached call runs
only on a full fill, so the gateway answers anything less with `PartialFillNotAllowed`, and a second
bid could never add to the first.

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

## Telling the bids apart

Bids on one incoming order share a commitment. `SolverAccount` derives a bid's nonce key from the
commitment, the session key and the op's own calldata, so every bid is the first sequence of a key
no other bid shares (`evm/docs/ai/changelog/2026-09-21-every-bid-has-its-own-nonce-key.md`). The
bids execute independently: none waits on another, and one that is never selected strands nothing.
`prepareBidUserOp` reads the nonce for the bid's own key when it signs the bid, once its calldata
exists.

The same calldata hash is the bid's identity on Hyperbridge and here. `submitBid` files each bid
under `keccak256(callData)`, so a solver's bids on one order stand side by side there. A bid row
carries the identifier: retraction takes back every one recorded on the commitment, and
`claimReservation` takes either one bid's holds (named by
identifier) or every outstanding hold on the commitment, claiming row by row rather than taking the
newest and guarding on a reservation value two rows can share. Without that, one bid's settlement took
another's holds, and two bids holding the same amount against the same order had both rows cleared by
a guard that could not tell them apart.

The bids go out best offer first. Nothing on chain orders them any more, but it is the order they are
built and held in.

## What a bid's gas covers

The gas estimate is shared by every bid on the order, since the fill itself is the same, but each bid
prepends its own funding calls, which are not simulated. The estimate is therefore cached without a
funding allowance, and each bid adds `FUNDING_GAS_PER_CALL` for each of its own calls when it is
signed. Baked into the shared estimate, every bid carried whichever calls were cached when it was
taken.

## What a bid holds

Each bid reserves against its own limit order alone. A hold that cannot be taken drops that bid and
leaves the rest of them alone, where the all-or-nothing `holdAll` used to drop the whole order. A bid
row still carries `reservations` as a list of `{ limitOrderId, amount }`, and `claimReservation`
hands it over exactly once.

The profit gate runs per bid against that bid's own payout, so a tail bid worth less than its gas is
refused on its own rather than hidden inside a combined figure. What the strategy reports for the
order is what its bids earn together.

When the fill lands, one bid delivered and the rest can only revert, so exactly one hold is worked
down and the others come back. Which bid executed is not in the event, which names the shared
commitment rather than the op: the delivery says it instead. A bid is signed for its own payout and
the gateway clamps it to what was outstanding, so the hold matching the delivered amount is the bid
that filled, the closest hold at or above it is the next best answer, and a tie falls back to the
order the bids went out in.

Existing databases get `reservations` and `bid` columns; the two columns `reservations` replaces
are left behind rather than migrated, since a hold outlives its bid by minutes and nothing reads a
settled one.
