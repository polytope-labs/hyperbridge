# 2026-09-12 — Index gateway OrderCancelled events (#1185)

The Intent Gateway's `OrderCancelled(commitment, canceller)` event now has an indexer handler. The indexer records every cancellation initiation, including the initiating chain and account, and exposes `CANCELLED` while a refund is pending. `EscrowRefunded` remains the terminal `REFUNDED` transition.

The current gateway ABI already declared this event, so the refresh retains that single declaration instead of adding a duplicate.

Files: `src/configs/schema.graphql`, `src/services/intentGatewayV3.service.ts`, `src/handlers/events/intentGatewayV3/orderCancelledV3.event.handler.ts`, `src/mappings/mappingHandlers.ts`, `scripts/templates/evm-chain.yaml.hbs`.
