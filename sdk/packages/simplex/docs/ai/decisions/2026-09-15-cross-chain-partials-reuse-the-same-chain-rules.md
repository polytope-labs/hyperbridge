# 2026-09-15 — Cross-chain partials reuse the existing partial-fill rules

Decided: extend `FXFiller`'s existing partial-fill rules to cross-chain orders, rather than port the
scoring the #980 branch had. Seun chose this while rebasing #980.

The branch predated main's partial-fill work. It scored a slice that does not complete the order as
FX margin minus execution cost, and refused it when that was negative. Main scores every partial on
its gross USD edge and exempts it from the profit floor, because the engine cannot see the margin
built into the operator's curve.

Rejected: the branch's net scoring. It would judge cross-chain and same-chain partials by different
rules, although both collect no fees and both release escrow pro rata.

## The fill progress is read before sizing

Decided: read `_partialFills` for every order before any leg is sized. A failed read skips the order.

Main read it lazily, only once an under-fill was on the table, and refused any order that had one.
Sizing against the remainder needs the value up front. One read per output is cheap next to the
balance reads, venue quotes and gas simulation an evaluation already makes.

Rejected: keeping the lazy read. A full-size bid on a started order never consulted it, so that bid
was priced against the whole escrow.

## One release formula for both paths

Decided: price each leg's released escrow with `cumulativeReleased`, on both paths.

Cross-chain releases exactly that amount. Same-chain releases `input × fill / total` per slice and
sweeps what is left on the completing fill. The two agree to within integer dust. On a completing
fill the cumulative figure is never the higher one.

Rejected: an exact same-chain formula. It needs the escrow balance from `_orders`, a second read,
to recover a few wei.
