# Phantom bid calldata decoding (`extractFillDataVm2`)

Verified by executing the function against both shapes; the selector check is what makes the two-interface
attempt necessary.

1. `handlePhantomOrderPrices.handler.ts` injects `extractFillDataVm2` into `aggregatePhantomBids` as
   `extractFill`. The SDK's own `extractFillData` is not used here: it decodes with viem, whose byte handling
   throws inside SubQuery's VM2 sandbox.
2. A bid's `callData` is the solver account's ERC-7821 `execute(mode, executionData)` batch. The batch is decoded,
   and each call whose `target` is the gateway is a `fillOrder` candidate. The bid's sender must be
   EIP-7702-delegated to one of the chain's `SOLVER_ACCOUNT_ADDRESSES`; the list carries the current
   SolverAccount and, during a redeployment, the one it replaced. The bid's sender must be
   EIP-7702-delegated to one of the chain's `SOLVER_ACCOUNT_ADDRESSES`; the list carries the current
   SolverAccount and, during a redeployment, the one it replaced.
3. `decodeFillOrderEither` tries the v2 interface (`FILL_ORDER_ABI`, with `validUntil`, selector `0xa5470064`)
   and then the v1 one (`FILL_ORDER_V1_ABI`, selector `0x5cfb1ea5`). ethers validates the selector before
   decoding, so exactly one can match and there is no payload that could be mis-decoded as the other shape.
   Which one a bid carries depends on the gateway it targets — solvers encode for the deployment they bid
   against, and gateways predating `validUntil` take v1.
4. A call matching neither shape is skipped, and if no call in the batch decodes the function returns null.
5. **Null is dropped silently upstream** — `aggregatePhantomBids` does `if (!fillData) continue` with no log. A
   decoding regression therefore shows up as missing pool rates rather than as an error, which is why the shapes
   are covered by tests rather than left to runtime observation.
