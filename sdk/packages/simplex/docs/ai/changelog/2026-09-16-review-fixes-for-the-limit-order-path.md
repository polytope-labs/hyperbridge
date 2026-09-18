# 2026-09-16 — Review fixes for the limit order path

Four defects found reading the stack back.

## An expired limit order is now swept off the book

`expiresAt` was read in one place, the matcher, so an order that outlived it stopped matching and
nothing else changed: it stayed `open` and reconciliation put the
posting back whenever it lapsed. The orderbook went on advertising depth that the filler would always
refuse.

`LimitOrderService.expireStale()` withdraws those postings and moves the row to a new `expired`
status. The lifecycle runs it on its own 30 second clock. An `expiresAt` that cannot be parsed counts
as no expiry, which is what the matcher already does with it.

## The matcher ranks on what an order pays, not what it quotes

`matchLimitOrder` ranked on `offer` and used `available` only to break ties, so an order quoting a
wonderful rate with almost nothing left beat one that could cover the swap outright. The payout is
`min(offer, remaining - reserved)`, so the winner paid what little it had and the caller skipped a
cross-chain fill that was there to be had. Ranking is on `payout` now, with the same tie-break.

## A short offer is a match, and the caller decides

Candidates were filtered by `offer >= requestedOutput` before ranking. That made a same-chain partial
fill impossible whenever the price rather than the size was what fell short, though the same
shortfall coming from `remaining` reached the partial-fill logic in `fx.ts` and was handled there.
The filter is gone; what remains is that an order with nothing left to pay is no match. Cross-chain
still skips on any under-fill, which is the gateway's rule, not the matcher's.

## A hold could be given back twice

`IntentFiller`'s catch around `executeOrder` released the reservation directly. The bid row is
written inside that same try, and the code after it emits on the monitor where a consumer's listener
can throw, so on that path the row existed carrying the hold and released it again when the bid was
retracted, freeing capacity another bid was holding. The catch now claims through the bid row once
one is on its way, and releases directly only before that. A write that fails strands the hold rather
than releasing it twice: an overstated reservation refuses fills, an understated one oversells.
