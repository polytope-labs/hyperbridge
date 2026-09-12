# 2026-09-12 — Cancellation status is guarded locally (#1185)

Chosen: `recordOrderCancellation` changes an already-indexed order from `PLACED` to `CANCELLED` only. `EscrowRefunded` remains the owner of the terminal `REFUNDED` status.

A same-chain cancellation emits `OrderCancelled` before `EscrowRefunded` in one transaction. A cross-chain cancellation can emit `OrderCancelled` on one datasource while the source-chain refund is indexed independently. The local guard prevents a delayed cancellation log from regressing a previously refunded order.

Rejected: changing the established pending-status or general `updateOrderStatus` behavior. Those paths predate this change and cover fills, releases, refunds, and their side effects; this cancellation addition must preserve their production behavior.
