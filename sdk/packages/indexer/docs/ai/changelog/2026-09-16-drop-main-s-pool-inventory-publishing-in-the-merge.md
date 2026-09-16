# 2026-09-16 — Drop main's pool inventory publishing when merging main

Merging `main` into the solver-inventory branch collided on the pool-inventory publishing this branch removes:
`main` kept extending it while the branch deleted it. Every conflict resolves the same way — `main`'s feature,
without the publishing.

- **`escrowReleasedV3` handler.** `main` moved the REDEEMED decision into `recordEscrowRelease`, which now takes the
  event's `solver` and decides between a completing release and a non-finalizing partial redeem. This branch still
  called `updateOrderStatus(REDEEMED)` explicitly, which predates that. Kept `main`'s handler, minus the
  publication; keeping both would have forced REDEEMED on partial redeems.
- **`escrowReleasedV3Legacy` handler**, new on `main`. Its publication block goes. It was the only caller of
  `IntentGatewayV3Service.filledBeneficiary`, which resolved the beneficiary from the gateway's `_filled` mapping
  purely to attribute the publication, so that method goes too. The handler still records the release, passing an
  undefined solver as `main` wrote it.
- **`intentGatewayV3.service`.** Removed `publishInventoryAfterFill`, `publishInventoryAfterEscrowRelease` and
  `filledBeneficiary`.
- **`fillEnrichment.handlers.test`**, new on `main`. Dropped the `publishInventoryAfterFill` spy and its two
  call-count assertions; the rest of the fill-enrichment coverage is unchanged.
- **Pool liquidity refresh flow doc.** Stays deleted, as does the pool-publication step `main` added to the
  intent-gateway volume flow.

Everything else `main` added merged unchanged: cross-chain partial fills, userop hashes, delivered amounts and fee
token, `OrderCancelled` indexing, and protocol fee refunds.

Files: `src/handlers/events/intentGatewayV3/escrowReleasedV3.event.handler.ts`,
`src/handlers/events/intentGatewayV3/escrowReleasedV3Legacy.event.handler.ts`,
`src/services/intentGatewayV3.service.ts`, `src/services/__tests__/fillEnrichment.handlers.test.ts`,
`docs/ai/flows/intent-gateway-volume-indexing-orderfilled.md`,
`docs/ai/flows/pool-liquidity-refresh-orderfilled-partialfill-escrowreleased.md`
