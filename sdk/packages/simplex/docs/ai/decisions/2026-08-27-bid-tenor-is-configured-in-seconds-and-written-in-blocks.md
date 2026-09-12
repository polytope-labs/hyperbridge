# 2026-08-27 — Bid tenor is configured in seconds and written in blocks

Chosen: `bidValiditySeconds` (default 300) is converted to a block height per destination chain in
`ContractInteractionService.bidValidUntilBlock()`.

The on-chain field has to be blocks: `fillOrder` compares it against `_blockNumber()`, the same clock as
`order.deadline`, and having the two disagree about what "expired" means would be a trap. But blocks are the wrong
unit for an operator — 5 minutes is 25 blocks on Ethereum and 150 on Base, so a single configured block count would
mean something different on every chain, and price risk is denominated in time, not blocks. Converting at the edge
keeps both sides in their natural unit. Rounding is deliberately up: erring long costs a slightly stale quote, erring
short silently drops bids we would have won.

300 seconds is chosen to cover the quote-to-fill path and little more: cross-chain confirmation waits reach roughly
180s on the deepest default policies, so 5 minutes clears the mechanical part of the round trip while keeping the
window over which the quoted price is a firm commitment short. On a volatile pair that is the number that matters —
optionality handed to the placer scales with time, and USDC/CNGN reprices in steps rather than drifting.

The cost is on the other side and worth watching. A bid is not consumed when signed: it waits in the coprocessor
until the placer selects a solver, so an expiry shorter than the selection window is lost revenue rather than lost
money. `BID_TTL_MS` (the 1-hour retraction sweep) is the only stated expectation in the tree for how long selection
takes, and 5 minutes sits well inside it — if bids are observed lapsing unselected, this is the knob, and raising it
is a pricing decision rather than a correctness one.

It is deliberately not tied to `BID_TTL_MS`: that TTL governs reclaiming a deposit, this governs how long we are
short an option, and the two should not be assumed to move together.

The window carries a 30-second discovery allowance on top of the configured validity, added before the conversion so
the rounding happens once. The head is read before the bid is built, signed, submitted to Hyperbridge and finally
discovered and selected, so it is already behind by the time a fill lands, and `Chain.blockTime` is nominal rather
than exact.

It is denominated in seconds rather than blocks because that lag is wall-clock and does not scale with block time —
a flat block count would be worth 60s on Ethereum and 1.25s on Arbitrum for the same real delay, which is backwards.
The asymmetry decides the direction: overshooting costs a marginally staler quote, undershooting silently throws
away bids that were about to be won.

Alternatives rejected:

- _Derive it from the order's own deadline._ Placer-controlled with no ceiling — the input being defended against.
- _Reuse the 1-hour bid TTL._ Conflates deposit housekeeping with price risk; an hour of free optionality on a naira
  pair is worth real money.
- _Per-pair tenors._ Right in the long run — a stablecoin pair could safely carry a longer expiry — but one
  conservative global default fixes the exposure now without adding a config surface to get wrong.

On a gateway predating the field the bound is dropped rather than the fill refused. Refusing would take the filler
off any chain not yet upgraded, which trades a bounded risk for a certain outage; the one-per-chain warning is there
so the gap is visible rather than assumed away.
