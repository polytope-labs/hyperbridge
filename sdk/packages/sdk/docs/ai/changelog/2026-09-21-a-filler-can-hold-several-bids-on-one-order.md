# 2026-09-21 — A filler can hold several bids on one order

A solver quoting from several resting prices sends one bid per price on the same order, and they have
to be able to execute independently. Two things stood in the way.

**One nonce key per order.** `SolverAccount` bound a bid's nonce key to `(commitment, sessionKey)`, so
a solver's bids on one order were sequences of a single key, which the EntryPoint runs strictly in
order: one bid that was never selected blocked the rest. The key now also commits to the op's
calldata — `uint192(keccak256(commitment ‖ sessionKey ‖ keccak256(callData)))` — so each bid is
sequence 0 of its own key (`evm/docs/ai/changelog/2026-09-21-every-bid-has-its-own-nonce-key.md`).
`CryptoUtils.bidNonceKey(commitment, sessionKey, callData)` derives it, and `BidManager` and the
phantom bid verifier check it.

**One bid per filler on Hyperbridge.** `pallet-intents-coprocessor` kept one bid per
`(commitment, filler)`: `place_bid` replaced the previous one, and the RPC's pool cache and
`RpcBidInfo`'s ordering did the same. Bids now live in `OrderBids`, keyed
`(commitment, filler, bid)`:

- `place_bid(commitment, bid, user_op)` and `retract_bid(commitment, bid)`. `bid` is an `H256` the
  filler chooses to tell its bids on an order apart; by convention it is `keccak256(callData)`
  (`CryptoUtils.bidId`), the same hash the nonce key takes. Placing again under an identifier the
  filler already holds replaces that bid alone, and each bid holds its own deposit.
- `BidPlaced` and `BidRetracted` carry the `bid`. The offchain key is
  `intents::bid:: ++ commitment ++ filler ++ bid`.
- `intents_getBidsForOrder` returns every bid with its `bid`, and the pool cache replaces only on the
  same `(filler, bid)`.
- `OrderBids` is a new storage item rather than a migrated `Bids`: the new key layout lives under its
  own prefix, so an entry of the old shape is never read as one of the new. Nothing is migrated, and
  a bid standing in the old `Bids` map at the upgrade is neither served nor retractable through the
  new calls.

In the SDK, `IntentsCoprocessor.submitBid(commitment, userOp, bid)` and `retractBid(commitment, bid)`
take the identifier explicitly, and `FillerBid`, `BidStorageEntry` and the helpers' `RpcBidInfo` report
it. The phantom helpers keep one bid per filler under the zero identifier.

The indexer's `FillerBid` gains `bid`, read from `BidPlaced`, and bid data is matched to its extrinsic
or RPC entry on commitment and identifier.

Simplex signs each bid with the nonce its own key reports (`EntryPoint.getNonce`), files it under
`keccak256(callData)`, and retracts every identifier its account holds on a commitment, read back
from the pallet's storage.
