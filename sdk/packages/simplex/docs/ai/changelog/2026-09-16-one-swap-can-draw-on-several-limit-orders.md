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

Bids on one incoming order share a commitment, so `bidNonceKey(commitment, session)` gives them one
ERC-4337 nonce key and only the 64-bit sequence can distinguish them. `estimateGasFillPost` reads the
base nonce once and caches it, and `getNonce` does not move until an op executes, so signing every
bid with the cached value would leave all but the first failing EntryPoint validation with AA25. The
i-th bid is signed with `base + i`.

That sequence is also the bid's identity. A bid row carries it, and `claimReservation` takes either
one bid's holds (named by sequence) or every outstanding hold on the commitment, claiming row by row
rather than taking the newest and guarding on a reservation value two rows can share. Without that,
one bid's settlement took another's holds, and two bids holding the same amount against the same
order had both rows cleared by a guard that could not tell them apart.

The EntryPoint consumes a key's sequences in order with no gaps, so a bid that never executes strands
every bid behind it. Signing order is execution order here — best price first, which is the order the
walk takes them in — and a number is spent rather than counted off: a bid whose submission fails
leaves its number to the next bid. A pooled submission counts as spent, since it may still land and
two bids sharing a sequence is the worse failure.

That covers every drop simplex can see as it sends. A bid that is accepted but never executes — not
selected, reverted in validation, expired past its `validUntil` — frees a number we have already
signed past, and recovering it means retracting the bids behind it and re-signing them. Whether to
build that or give bids distinct keys is the question #1259's walk decides, since the commitment-
derived key rules the second out today (`BAD_NONCE_BINDING` in `verify.rs`).

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

Existing databases get `reservations` and `sequence` columns; the two columns `reservations` replaces
are left behind rather than migrated, since a hold outlives its bid by minutes and nothing reads a
settled one.
