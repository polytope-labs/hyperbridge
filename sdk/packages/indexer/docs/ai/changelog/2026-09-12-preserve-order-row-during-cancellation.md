# 2026-09-12 — Preserve the order row during cancellation (#1185)

Cancellation now writes its event and status metadata only. The earlier local guard could still overwrite a refund from an independent indexer holding a different cached snapshot. Missing parents follow the established pending-metadata behavior, without changing lifecycle updates or their side effects.

Eight service tests cover existing and missing parents, all terminal statuses, repeated logs, pending metadata, and same-chain refunds. A real SubQuery/Postgres regression test reproduces the unsafe write and checks both indexers' flush orders.

Files: `src/services/intentGatewayV3.service.ts`, `src/handlers/events/intentGatewayV3/orderCancelledV3.event.handler.ts`, `src/services/__tests__/intentGatewayV3.cancellation.service.test.ts`, `scripts/tests/verify-cancellation-concurrency.cjs`.
