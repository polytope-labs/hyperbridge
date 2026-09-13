# 2026-09-13 — Optional placement userOpHash (#1112)

Added nullable, indexed `IOrderV3.userOpHash` alongside the placement transaction hash. Both current and legacy `OrderPlaced` handlers can resolve the operation from the transaction receipt. Missing receipts, direct placements, and uncertain attribution leave it unset; failed enrichment does not prevent placement indexing. Replaying an existing row can populate the field but cannot erase a known hash when receipt enrichment is unavailable.

Moved the existing fill operation matcher into a shared helper without changing its matching behavior. Added placement coverage for execution boundaries, multiple operations and placements, sender mismatches, current/legacy handler persistence, and unavailable receipts. Extended the additive migration test to cover the seventh nullable field.

Files: `src/configs/schema.graphql`, `src/handlers/events/intentGatewayV3/orderPlacedV3.event.handler.ts`, `src/services/intentGatewayV3.service.ts`, `src/utils/userOp.helpers.ts`, `src/utils/fill.helpers.ts`, `src/utils/__tests__/userOp.helpers.test.ts`, `src/services/__tests__/fillEnrichment.handlers.test.ts`, `scripts/tests/verify-fill-schema-migration.cjs`.
