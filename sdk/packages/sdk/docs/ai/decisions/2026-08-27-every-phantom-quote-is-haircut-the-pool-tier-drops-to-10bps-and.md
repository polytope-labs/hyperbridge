# 2026-08-27 — Every phantom quote is haircut; the pool tier drops to 10bps and wallet quotes pay 5bps

Chosen: `UNISWAP_QUOTE_HAIRCUT_BPS` becomes 10bps and a new `PHANTOM_QUOTE_HAIRCUT_BPS` of 5bps is charged to every bid that does **not** declare Uniswap V4 positions. A bid pays one haircut or the other, never both — `poolPriced` selects which function runs, and the structure from 2026-08-24 (keyed off the signed declaration, applied to the quote before the zero-check and the median) is unchanged.

Alternatives considered:

- **Stacking the 5bps on top of the pool haircut.** Rejected: the two are the same kind of adjustment — margin between what a solver names and what the protocol is willing to publish — and the pool tier is already the larger of the two. Compounding them would price a pool-funded bid at 15bps for reasons no single rationale explains, and would make the shipped number harder to reconcile against the fee tier it is meant to track.
- **Leaving wallet-funded quotes unhaircut.** Rejected: a wallet quote is still the most optimistic number a solver names at bid time, and the median it feeds is published to takers as a rate they can trade against. A small uniform margin is what keeps the published rate on the executable side of the bid.
- **Applying the base haircut after the median instead of per quote.** Rejected for the same reason as before: it is now a haircut every bid pays, so applying it to the aggregate would be arithmetically similar but would split one rule across two places in the code.

The 5bps also applies to a bid whose declaration is absent or unparseable, which is the correct reading — no declaration is no claim to be pricing off a pool.
