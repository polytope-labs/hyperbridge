# 2026-09-07 — A phantom bid's accepted sources are derived from the chain set, not configured

Chosen: drop the optional `simplex.acceptedSourceChains` key and derive the declaration in `IntentFiller` at bid
time as every configured chain, watch-only ones included. `preparePhantomBidUserOp` always encodes a declaration.

Why: the set of source chains a filler serves is already fully determined by the chains it is configured on —
the event monitor only sees orders sourced there, and `evaluateOrder` already accepts a watch-only source as
long as the destination is live. Keeping a second, hand-typed list of the same chains meant it drifted or, as on
mainnet, was never set at all — and an absent declaration is read downstream as "any chain", which advertised
routes the filler had never agreed to and produced no `PoolRoute` rows for the depth it did offer.

Alternatives considered:

- **Default the key to the derived list but keep it overridable.** Rejected: there is no case where a filler
  wants to accept an order sourced on a chain it does not watch (it would never see it), nor refuse one sourced
  on a chain it does; an override can only make the declaration wrong.
- **Derive once at boot.** Rejected: chains are added and removed while the filler runs, and every path that
  changes them already goes through state the bid can read.
- **Exclude watch-only chains.** Considered and reversed the same day. Watch-only governs whether the filler
  commits inventory on a chain as a *destination*; a cross-chain order sourced on a watch-only chain is still
  filled on a live destination with the escrow released to the filler there, so the source is acceptable.
  Excluding it would have hidden real routes from takers.
