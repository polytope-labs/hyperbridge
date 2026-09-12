# Intent gateway order cancellation (OrderCancelled to EscrowRefunded)

1. `IntentGatewayV2.cancelOrder` emits `OrderCancelled(commitment, canceller)` after all paths that would revert have been checked. The event is emitted on the chain where cancellation begins.
2. `handleOrderCancelledEventV3` resolves the event timestamp and calls `IntentGatewayV3Service.recordOrderCancellation`. It creates an `IOrderV3Cancellation` row identified by `{transactionHash}.{logIndex}` so repeated cancellation attempts remain visible.
3. For an indexed order at `PLACED`, the service records `CANCELLED`. It does not overwrite any other status, particularly an already-indexed `REFUNDED` from the independent source-chain datasource.
4. A same-chain cancellation emits `EscrowRefunded` later in the same transaction, so the status ends as `REFUNDED`. A cross-chain cancellation can remain `CANCELLED` until Hyperbridge delivers the source-chain refund; a source-side cancellation may remain there while its GET request awaits a response.
5. If the order has not been indexed yet, the established `PendingStatusMetadata` workflow handles the status record. This handler deliberately follows that existing policy without changing its ordering or side effects.
