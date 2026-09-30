# 2026-09-12 — Cancellation status stream (#1185)

Added `OrderStatus.CANCELLED` for indexer cancellation metadata. The order status stream continues polling after cancellation, prefers existing settled statuses over a later cancellation entry, and remembers the last emitted status to avoid repeated cancellation emissions. Six focused tests exercise the stream and parser with mocked query responses; these are not live GraphQL compatibility tests.

Files: `src/types/index.ts`, `src/protocols/intents/IntentGateway.ts`, `src/tests/orderStatusStream.test.ts`.
