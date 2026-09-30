# 2026-08-27 — The surplus split is a helper because `_fillSameChain` sits on the via-ir stack limit

Chosen: the overpayment split in `_fillSameChain` moved into a `private view` `_splitSurplus`.

This is not a readability change. Removing the `recordSpread` call from the end of `fillOrder`
broke compilation with "Variable var_protocolShare_1 is 1 too deep in the stack" — and bisecting
showed the `validUntil` check was not the cause. That oracle call had been keeping `commitment`
and the order's fields live past the fill, which happened to give the optimizer a stack layout
that fit; taking it out took the slack with it.

So the function was already one slot from the limit and only compiled by accident of an unrelated
call site. Extracting the split gives the two share variables their own frame and restores the
headroom, without touching the fill arithmetic — the helper is a literal transcription of the
branch it replaces.

Worth knowing for anyone editing `_fillSameChain`: it has essentially no stack budget left, and
changes elsewhere in `fillOrder` can push it over from a distance.
