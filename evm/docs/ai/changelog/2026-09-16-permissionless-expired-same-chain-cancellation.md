# Permissionless expired same-chain cancellation

Same-chain Intent Gateway orders can now be cancelled by the order user through the deadline and by any caller once `_blockNumber() > order.deadline`. The caller pays transaction gas and receives no escrow; the gateway refunds the original `order.user`, including only the remaining balance after any partial fills. `OrderCancelled` records the caller, and `EscrowRefunded` confirms the refund in the same transaction.

The existing SDK `IntentGateway.cancelOrder(order, indexerClient, options?)` flow handles this route with zero native dispatch value and zero relayer fee. It preserves `order.user` in calldata, ignores the cross-chain route choice for same-chain orders, and accepts either the caller's signed raw transaction or an already-broadcast transaction hash. Integrations must persist the finalized post-fee `Order`, submit from a wallet on the order's chain, and treat `EscrowRefunded` or on-chain refund state as the completion signal.

Deployments must upgrade the gateway implementation and intrinsic module before keepers can use the new authorization rule. `IntentGatewayScript` currently prints upgrade calldata containing `migrate()` unconditionally: use empty init data when the target proxy already reports `version() == 3`; older proxy versions require the printed migration call.
