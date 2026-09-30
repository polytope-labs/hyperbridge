# Cross-chain partial fills (#980)

- `EscrowReleased` now carries the solver, stored in `IOrderV3EscrowRelease.solver`. It is null for
  the old event shape.
- The old shape, `EscrowReleased(bytes32,(bytes32,uint256)[])`, is still indexed, by a new unfiltered
  handler, `handleEscrowReleasedEventV3Legacy`.
- An order becomes `REDEEMED` only once every input is fully released. Each release adds to
  `IOrderV3InputAsset.released`.
- Each fill adds its outputs to `IOrderV3OutputAsset.filled`.
- A refund of all-zero amounts no longer marks the order `REFUNDED`. Cancelling an order that was
  already fully filled emits exactly that.
- Filler volume, gateway `FILLED` volume and filler points are credited per fill and per partial fill,
  valued from that fill's outputs. Before, the completing filler was credited with the whole order.
- A fill indexed before its order is replayed into `filled` and filler points when the order is placed.
- Fill, partial-fill and release rows now double as markers, so a replayed log is skipped rather than
  counted twice.
- Pool inventory after a release is published for the event's solver.

The schema changes are additive and nullable. No backfill is performed.

Files: `scripts/templates/evm-chain.yaml.hbs`, `src/configs/abis/IntentGatewayV3.abi.json`, `src/configs/schema.graphql`, `src/handlers/events/intentGatewayV3/escrowReleasedV3.event.handler.ts`, `src/handlers/events/intentGatewayV3/escrowReleasedV3Legacy.event.handler.ts`, `src/handlers/events/intentGatewayV3/escrowRefundedV3.event.handler.ts`, `src/mappings/mappingHandlers.ts`, `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/fillEnrichment.handlers.test.ts`
