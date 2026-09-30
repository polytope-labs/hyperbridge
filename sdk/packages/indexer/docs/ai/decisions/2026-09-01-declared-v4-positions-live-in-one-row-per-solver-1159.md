# 2026-09-01 — Declared V4 positions live in one row per solver (#1159)

Chosen: `SolverV4Positions`, keyed by the solver's address, holding the tokenIds from its latest bid and the
chain that bid was for. Written by the phantom snapshot, read by the refresh with a single keyed `get`.

Alternative rejected — a row per (chain, tokenId). It was implemented first and is the wrong grain: the refresh
asks "what does this solver declare", so a per-position table makes that a filtered scan per provider per chain,
and a position that changes hands or disappears needs its own reconciliation. One row answers the question the
readers actually ask, and replacing it wholesale each window is the reconciliation.

Alternative rejected — no storage at all, decoding the declaration back out of `FillerBid.bidData` (which does
hold every bid's raw userOp). It was implemented too, and it needs no schema, but the read is a walk over recent
phantom orders, then their bids, SCALE-decoding each one to find the sender — per solver, per refresh. The
declaration is a small, current fact; storing it decoded is what makes reading it cheap.

Alternative rejected — carry the last sweep's position VALUE forward instead of the tokenIds. Wrong in the case
that matters: simplex funds fills out of these positions, so a fill drains the position inside the fill
transaction while wallet and vault balances barely move, and a carried value keeps advertising precisely the
inventory the fill just spent. The tokenIds are stable; the value is not, so the value is always re-read.

One row per solver assumes a solver declares on one chain, which holds while Uniswap V4 is configured for a
single chain (Base). That assumption is load-bearing, so the writer refuses to overwrite a row recorded on
another chain and warns instead — the key has to grow to (chain, solver) the day a second V4 chain is
configured, and a silent overwrite would have deleted real inventory in the meantime.

Positions are recorded after the aggregation's ownership check, not as declared — a declaration is a pointer,
not a claim — and the refresh checks the owner again when it re-reads, because a row recorded last window cannot
know the position has since been sold.

The reconciliation is driven by the aggregation's `solvers` — every verified bidder — and not by the solvers
appearing in `lpBalances` or `positions`. Both of those are filtered by what the solver turned out to hold or
declare, so a bidder holding nothing anywhere and declaring nothing appears in neither, and its previous
declaration would never be emptied. That bidder is precisely the V4-funded profile whose whole inventory sits in
the positions it just stopped offering, so it is the case that matters most (review of #1194).
