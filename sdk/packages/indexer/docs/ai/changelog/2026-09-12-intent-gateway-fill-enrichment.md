# 2026-09-12 — Intent gateway fill enrichment

Intent Gateway V3 fills now expose the ERC-4337 user operation hash and the delivered ERC-20 amount, while placed
orders retain the host fee token and its decimals at the placement block. The added columns are nullable so the
substrate node's in-place migration preserves existing rows without a reindex.

Files: `src/configs/schema.graphql`, `src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`, `src/handlers/events/intentGatewayV3/partialFilledV3.event.handler.ts`, `src/handlers/events/intentGatewayV3/orderPlacedV3.event.handler.ts`, `src/services/intentGatewayV3.service.ts`, `src/utils/fill.helpers.ts`, `src/utils/host.helpers.ts`, `src/utils/rpc.helpers.ts`, `scripts/tests/verify-fill-schema-migration.cjs`.
