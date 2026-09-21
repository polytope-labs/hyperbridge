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
