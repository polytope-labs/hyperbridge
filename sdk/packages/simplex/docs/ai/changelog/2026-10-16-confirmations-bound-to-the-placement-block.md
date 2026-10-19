# 2026-10-16 — Confirmations are bound to the block the order was scanned from

A cross-chain order is now filled only while a quorum of the source endpoints that answer
still places its `OrderPlaced` transaction in the block the scanner read it from, and the
source escrow is re-read under such a quorum right before bidding. A placement that moves during the
wait is skipped instead of bid on.

## Why

`IntentGatewayV2.placeOrder` numbers each order with a monotonic `_nonce`, and the
commitment (`order.id`) hashes the whole order, nonce included. A source reorg that
re-mines the placement in a different order — behind an attacker's own placement, say —
keeps the transaction hash but changes the nonce, and so changes the commitment. The
confirmation poller counted depth from the receipt alone and never checked which block
the quorum agreed on, so the filler waited out a re-mined transaction, bid on the stale
commitment, and the destination paid the output for an order whose redeem then reverted
`UnknownOrder` on source: one full output lost.

## What changes

The scanner's `OrderPlaced` log carries its `blockHash`, and `newOrder` now passes it on.
The confirmation wait compares the block a quorum agrees the transaction is in against
that scanned block on every poll; a mismatch stops the wait and the order is skipped
(`Source placement reorged`). A cross-chain order the scanner reported without a block is
skipped too (`Placement block unknown`) — without it there is nothing to bind the depth
to.

After the wait and before bidding, the source escrow is re-read through the quorum
(`readWithQuorum`): each endpoint votes on the whole escrow vector, and a bid goes out
only when a quorum agrees every leg still holds escrow. Endpoints that fail do not vote
(the bar is the BFT threshold over those that answer), so this is not a floor of
`floor(2N/3)+1` of the configured set. An empty escrow skips the order (`Source escrow
missing after confirmations`); an escrow that could not be read at all skips it under its
own reason (`Source escrow unverifiable after confirmations`), so an RPC fault is not
reported as a phantom commitment.

## Interfaces

- `newOrder` carries `blockHash` alongside `order`, `transactionHash` and `graffiti`, and
  so does the public `order:detected` event (`SimplexEvents` gains `blockHash`).
- A custom `OrderScanner` must set `ScannedOrder.blockHash`: an empty value skips every
  cross-chain order (`Placement block unknown`).
- `QuorumPublicClient.getTransactionInclusion` returns the `(blockHash, blockNumber,
confirmations)` a quorum agreed on; `getTransactionConfirmations` is its depth.
- `QuorumPublicClient.readWithQuorum(label, read)` votes each endpoint's whole read value.
- `aggregateInclusion` is the BFT aggregation both callers share.
- `verifyOrderOnSource(order, quorum?)` reads all legs in one quorum call when a quorum is
  given, and returns `held`, `missing` or `unverifiable`.

## Constraints

- Fail closed: a cross-chain order missing its scanned block, or whose escrow no quorum
  agrees on (or can read) after the wait, is skipped.
- A placement re-mined in a different block is skipped even if its nonce — and so its
  commitment — did not change: the check is on the block, deliberately conservative.
- Same-chain orders and zero-confirmation curve points are unchanged: they still make no
  quorum read.
- The escrow re-read is at each endpoint's latest block, so it proves the commitment holds
  now, not that it held at the scanned block.
