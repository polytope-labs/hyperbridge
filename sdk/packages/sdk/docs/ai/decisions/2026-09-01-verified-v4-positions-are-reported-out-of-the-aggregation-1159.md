# 2026-09-01 — Verified V4 positions are reported out of the aggregation (#1159)

Chosen: `aggregatePhantomBids` returns the tokenIds it verified alongside the balances it swept, and
`readV4Position` (params object, with a `blockTag`) plus `positionAmountOfToken` are exported so a consumer can
re-value them later with the same reads and arithmetic a leg was weighted by.

Alternative rejected — let the consumer decode the bids itself. It is possible (the indexer stores every bid's
raw payload) and was tried: everything needed is already done here once — fetch the bids, verify each signature
and delegation, decode `paymasterAndData`, check ownership on-chain — so redoing it downstream duplicates the
security-relevant half of this module, and the two copies would drift.

Alternative rejected — have the consumer carry the last window's position VALUE forward instead of the tokenId.
It needs no new plumbing and is wrong in exactly the case that matters: simplex funds fills out of these
positions, so a fill drains the position inside the fill transaction while wallet and vault balances barely move,
and a carried value keeps advertising precisely the inventory the fill just spent.

Positions are reported after the ownership check, not as declared. A declaration is a pointer, not a claim, and
recording an unowned one downstream would hand the fill path a position to value that the solver cannot spend.

`solvers` is reported for the opposite reason: every other field is filtered by what the solver turned out to
hold or declare, so a verified bidder holding nothing anywhere is absent from all of them. A consumer
reconciling per-solver state ("this solver bid and declared nothing, so empty its row") cannot see it otherwise,
and that bidder is exactly the one whose inventory is all in positions it may have just stopped offering.
