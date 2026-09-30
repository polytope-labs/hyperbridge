# 2026-09-01 — Escrow releases and vault events refresh by provider, and depths re-sum from the store (#1159)

Chosen: `refreshProviderLiquidity(chain, provider, tokens)` beside the pool-scoped entry point, for the events
that move a solver's inventory without naming a pool.

Alternative rejected — resolve those events to pools and reuse the pool-scoped path. An escrow release names the
order, so its pools are resolvable, but a vault event names only a token; and re-reading every bidder of every
pool the solver touches costs an RPC per bidder to learn what one solver's balance did.

That entry point forced one change to the shared core, worth knowing about: the (pool, chain) depths are now
re-summed from the STORED bidder rows after the writes, not from the rows the refresh happened to re-read.
Summing the re-read subset was correct only because the pool-scoped path re-reads every bidder; with one solver's
rows in hand it would have erased everyone else's contribution.

Chosen: the escrow release resolves its filler from the gateway's `_filled(commitment)` mapping with one
`eth_call` at the event's block. `_withdraw` writes the beneficiary in the same call that emits the event, so the
mapping is authoritative from that block onwards, and the source chain's node never has to wait for the
destination chain's node to have indexed the fill.

Chosen: the vault refresh hangs off the end of `YieldVaultService.recordLedger` rather than off the handlers. The
"is this one of our solvers" gate and the duplicate-log guard already live there, and both are exactly the gates
the refresh wants.
