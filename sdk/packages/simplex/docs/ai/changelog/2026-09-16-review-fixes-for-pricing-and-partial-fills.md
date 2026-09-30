# 2026-09-16 — Review fixes for pricing and partial fills

## A repost acts on the cancel it asked for

`repost` threw the `CancelOrderResult` away and posted regardless. On a refusal, or on a request that
never got an answer, the old entry is still live and the new posting sits behind the same liability,
which is the one thing the resize path is built to avoid. It now goes ahead only on a cancel the
orderbook confirmed or its word that the entry is already gone; anything else leaves the row alone
with the reason on it for the next cycle.

## Cross-chain fills may be partial

`ExtrinsicIntents._fillCrossChain` keeps cumulative progress in `_partialFills[commitment][token]`,
clears `_filled[commitment]` on an under-fill so another solver can finish the order, and releases
escrow proportionally through `RedeemEscrowPartial`. The comment here still described the old
unconditional revert and `partialEligibleCheap` still required `sourceChain === destChain`, so a
cross-chain swap the operator was priced to serve was skipped whenever the payout fell short.

The output-calldata and prior-partial guards carry over unchanged, for the same reasons they hold
same-chain. One cost does not: a cross-chain partial pays the relayer fee carrying
`RedeemEscrowPartial` back to the source, and the partial's profit figure nets neither that nor gas,
so the operator's margin has to cover both. That is noted where the figure is computed.

## A lost guarded write is reported

`drawDown` and `release` guarded on the value they read and ignored whether the write applied, unlike
`reserve` three methods up. Only a second process on one `bids.db` can lose one, but losing it
silently leaves the order advertising output it has already paid. Both throw `LimitOrderWriteError`
now.

The two are also ordered rather than atomic: the draw-down goes first and the hold goes back after, so
a crash between them understates capacity until reconciliation instead of advertising output that is
gone.

## Smaller

`pnpm test:filler` named `fx.curve-payout.test.ts`, which this stack renamed, so the script failed
outright rather than skipping.
