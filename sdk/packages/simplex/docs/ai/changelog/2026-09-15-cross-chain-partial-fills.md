# Cross-chain and already-started partial fills in FXFiller (#980)

The gateway now accepts partial fills on cross-chain orders, and the filler follows.

- A cross-chain order the filler can only partly cover is bid as a partial. Before, it was skipped.
- An order other solvers have already partly filled is bid on. Before, an under-fill of it was
  refused, and a full-size bid on it was priced as if nothing had been filled.
- Before sizing, the filler reads each output's filled amount from the destination's `_partialFills`.
- Each leg is sized against what it still owes and the escrow it can still release. A started leg
  never sources more than it owes.
- A leg that is already complete goes out as a zero output.
- The escrow a fill releases is priced with the SDK's `cumulativeReleased`.
- Only an order carrying output calldata is still refused an under-fill.
- The bundled gateway ABI's `EscrowReleased` gains `solver`.

Scoring did not change. A partial scores its gross edge and is exempt from the profit floor. A fill
that completes the order must still cover execution from `order.fees`.

This replaces three commits from before the rebase (2b00c778, b45329ed, c4551898). They were
written against the older `StableFiller` and `FXFiller`.

Files: `src/strategies/fx.ts`, `src/services/ContractInteractionService.ts`, `src/tests/pairs.test.ts`, `src/config/abis/IntentGatewayV2.ts`
