# 2026-09-21 — A filler can hold several bids on one order

A solver quoting from several resting prices sends one bid per price on the same order. Those bids
are UserOps on one nonce key — `bidNonceKey(commitment, session)`, which `SolverAccount` checks — told
apart by the 64-bit EntryPoint sequence each one signs, and the EntryPoint runs a key's sequences in
order with no gaps.

`pallet-intents-coprocessor` kept one bid per `(commitment, filler)`: `place_bid` replaced the
previous one, and the RPC's pool cache and `RpcBidInfo`'s ordering did the same. A second bid from
the same solver silently evicted the first, leaving a bid signed at a sequence the chain could never
reach. Bids are now keyed `(commitment, filler, sequence)`:

- `place_bid(commitment, sequence, user_op)` and `retract_bid(commitment, sequence)`. Placing again at
  a sequence the filler already holds replaces that bid alone, and each bid holds its own deposit.
- `BidPlaced` and `BidRetracted` carry the `sequence`.
- The offchain key is `intents::bid:: ++ commitment ++ filler ++ sequence (u64 LE)`.
- `intents_getBidsForOrder` returns every bid with its `sequence`, and the pool cache replaces only on
  the same `(filler, sequence)`.
- `KeyBidsBySequence` (storage v2 → v3) moves each standing bid to sequence 0 with its deposit, so it
  stays refundable. Its offchain data cannot be moved by the runtime, so a bid standing across the
  upgrade stops being served over RPC until it is placed again.

In the SDK, `IntentsCoprocessor.submitBid(commitment, userOp, sequence)` and
`retractBid(commitment, sequence)` take the sequence explicitly, and `FillerBid`, `BidStorageEntry` and
the helpers' `RpcBidInfo` report it. The phantom helpers keep a single bid per filler at sequence 0.

The indexer's `FillerBid` gains `sequence`, read from `BidPlaced`, and bid data is matched to its
extrinsic or RPC entry on commitment and sequence, so two bids on one order in one batch are not
confused.
