# Cross-chain partial fills (#980)

The SDK follows the gateway's cross-chain partial fills.

- `RequestKind` gains `Execute = 5` and `RedeemEscrowPartial = 6`, matching `IntentsBase.sol`.
- A cancel from the source chain now proves each output's `_partialFills` slot instead of the order's
  `_filled` slot. `calculatePartialFillSlotHash` builds the keys. `encodeCancelFromSourceContext`
  encodes the GET context the gateway decodes: commitment, user, inputs and each output's total.
- `OrderStatusChecker.getFillProgress` reads per-output fill progress. `isOrderFilled` is now documented
  as finalized-only, because a partial fill clears `_filled`.
- `cumulativeReleased` mirrors the gateway's release formula, for solvers pricing a slice.
- The gateway ABI's `EscrowReleased` gains `solver`.
- `Bid.execute` and `OrderExecutor` document one fill lifecycle for both paths. Fills and their events
  land on the destination either way.

Files: `src/types/index.ts`, `src/utils.ts`, `src/index.ts`, `src/abis/IntentGatewayV2.ts`, `src/protocols/intents/OrderCanceller.ts`, `src/protocols/intents/OrderStatusChecker.ts`, `src/protocols/intents/Bid.ts`, `src/protocols/intents/OrderExecutor.ts`
