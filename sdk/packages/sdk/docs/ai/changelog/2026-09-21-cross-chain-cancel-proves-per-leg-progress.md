# Cross-chain cancel proves per-leg progress

A source-initiated cross-chain cancel has the gateway dispatch a GET for one storage key per order
leg, `_partialFills[commitment][index]` on the destination gateway, with the context
`abi.encode(commitment, user, inputs, totalRequired)`. The refund is each leg's escrow minus what
its credited output has already released.

`OrderCanceller` now matches that request. `fetchDestinationProof` proves every leg's slot, and
`quoteCancelFromSource` quotes a GET with the same keys and context. The slots come from
`partialFillSlot(commitment, index)` in `escrowReads.ts`, which mirrors the gateway's
`_calculatePartialFillSlotHash` with `_partialFills` at storage slot 11. Neither path calls
`calculateCommitmentSlotHash` any more.

`OrderStatusChecker.isOrderFilled` reads the `_filled` getter. It is true once the completing fill
or a cancellation has finalized the order, and false for an order that is only partly filled.
`getFillProgress(order)`, also on `IntentGateway`, returns the credited output per leg from
`_partialFills`, excluding surplus.
