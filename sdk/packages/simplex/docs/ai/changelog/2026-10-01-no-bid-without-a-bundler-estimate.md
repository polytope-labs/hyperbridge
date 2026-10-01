# 2026-10-01 — No bid without a bundler estimate, and none on an expired order

## A failed bundler estimate

When the bundler rejected `eth_estimateUserOperationGas` for a fill, `GasEstimator.estimateFillOrder`
logged a warning and returned fixed gas limits (500k call, 100k verification, 100k
pre-verification). Simplex cached those and bid with them. The bundler rejects the estimate
because the fill reverts in simulation, so the bid it produced reverted the same way when selected.

`EstimateFillOrderParams` takes `requireBundlerEstimate`. When it is set and a bundler is
configured, a failed estimate throws `Bundler gas estimation failed: <bundler error>` instead of
returning the fixed limits. It defaults to `false`, so `quoteOrderFees` and other SDK callers
still get a fee quote when the bundler is unavailable.

`ContractInteractionService.estimateGasFillPost` sets it. A failed estimate caches nothing and
throws, the FX strategy scores the order at zero, and no bid is sent. The service logs
`Error estimating gas, not bidding on this order` with the order id and the bundler's error.

## Chains with no configured call dispatcher

`GasEstimator.buildStateOverride` rewrites the gateway's params slot 5 with the chain's call
dispatcher address from `ChainConfigService`. BSC testnet, Gnosis Chiado, Pharos Atlantic and
Asset Hub Paseo have a gateway but no `Calldispatcher` in their config, so the value written was
12 bytes instead of 32. The bundler rejected every estimate on those chains as `Invalid params`,
and fills there only ever went out on the fixed limits.

When the config carries no dispatcher, the estimator now reads `params().dispatcher` from the
gateway, once per chain. Estimates on those chains succeed, so requiring one does not stop bids
there.

## An expired order

`order.deadline` is a block number on the destination chain, and `fillOrder` reverts `Expired()`
once that chain is past it. The placer fixes the deadline when the order is built, so an order
that is slow to land on its source chain can arrive already expired. Mainnet order
`0xc4440fb9…1d2a` (Polygon to Base) landed 11 Base blocks after its deadline.

`IntentFiller.evaluateOrder` reads the destination chain's latest block number before any strategy
prices the order. When `order.deadline <= head`, it logs
`Skipping order: its deadline has passed on the destination chain` with both numbers and emits
`orderSkipped` with the reason `Order deadline has passed on the destination chain`. A deadline
equal to the head counts as passed, because a fill lands in a later block.

When the head cannot be read, the order is evaluated as before. The gas estimate simulates the
same check and refuses an expired order.
