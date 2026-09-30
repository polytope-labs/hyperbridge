# 2026-09-21 — SolverAccount is not version-checked

#1292 removed `version()` from `SolverAccount`, so every deployment built from it, including the
testnet account `0x153DB990FE3b761B54ad71D0f1A0987dF11740DB`, reverts on the getter. The SDK still
required the solver's live delegation and the configured `SolverAccount` to report release 3. As a
result `prepareSubmitBid` refused every bid, and simplex could neither bid nor post limit orders,
failing with:

```
Fills are not supported by the destination gateway, live delegation, and configured SolverAccount
```

The release check now reads only the gateway:

- `supportsRateFills(client, gateway)` drops its `solverAccount` argument.
- `RateFillCapabilityReader` and `readRateFillCapability` take `(evmRpcUrl, gatewayAddress)`.
- `prepareSubmitBid` throws `Fills are not supported by the destination gateway` when the gateway is
  not on release 3.
- `GasEstimator` relies on `assertGatewayRelease` alone.

Phantom aggregation still rejects a bid whose sender is not delegated to a configured `SolverAccount`.

## Cross-chain order fees round their buffer up

`quoteOrderFees` adds 5% to fill gas plus relayer fee for a cross-chain order. The buffer was floored,
so with testnet gas priced at one unit (`convertGasToFeeToken` returns `1n` on testnets) the quote was
`(2 × 105) / 100 = 2`: exactly the solver's requirement. Simplex requires a strictly positive fee
profit, so it refused every SDK-placed cross-chain order on testnet. The buffer now rounds up,
`(sum × 105 + 99) / 100`, keeping the fee strictly above the requirement for any positive cost.


## Bids on a completed leg are not offered

For a multi-leg order, `OrderExecutor` kept offering bids whose only non-zero outputs were on legs
the destination had already credited in full. Executing one reverted in simulation with
`RateFillTooSmall`. Each round now drops a bid unless it pays into at least one leg that still has
an amount outstanding (`servesOpenLeg`). A bid whose outputs don't line up with the order's legs is
still offered.
