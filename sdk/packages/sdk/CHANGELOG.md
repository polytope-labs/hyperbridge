# @hyperbridge/sdk

## 2.7.0

### Minor Changes

- `IntentGateway.quoteOrderFees(order, options?)`: public quote for the solver fee, using the same policy `execute()`/`executeBest()` apply when `order.fees` is `0n`. Returns `{ fees, nativeValue, feeToken, estimate }` (`OrderFeesQuote` is exported) so integrators can check a user's fee-token balance and allowance, or native balance, before placing — instead of re-implementing the fee formula from `estimateFillOrder` components. `execute()` now derives its automatic fee from the same method.
- `AWAITING_PLACE_ORDER` now separates the placement transaction's native components: `value` carries the order's native-token input amounts (previously it carried the auto-quoted fee), and the new `nativeFee` field carries the native amount that funds `order.fees` (`0n` when the caller set `order.fees`). Sign with `value + nativeFee` to pay the fee in native token — the sum is also correct on the fee-token rail.
- Cross-chain cancellation relayer fees (source and destination routes) are now sized from an 800k source-chain gas budget, up from 400k, so refund deliveries clear on expensive source chains. Same-chain cancellations remain free of relayer fees.

## 2.6.1

### Patch Changes

- `SubstrateChain.stateMachineUpdateTime` reads pallet-ismp's `BoundedStateMachineUpdateTime` storage map directly instead of the `ismp_queryStateMachineUpdateTime` RPC. An evicted height now surfaces as a `MissingConsensusUpdateTimeError` from the absent storage entry rather than from matching an RPC error message.

## 2.6.0

### Minor Changes

- Cross-chain `order.fees` now attaches (fill gas + a `RELAYER_MESSAGE_GAS` (1M) settlement uplift) with a 5% buffer over the whole sum, while the solver-side requirement carries no padding — SDK-placed orders always clear a solver's fee gate, including expensive-source/cheap-destination routes that were previously refused. `RELAYER_MESSAGE_GAS` is exported.
- The cross-chain dispatch is always paid in the fee token: `estimateFillOrder` no longer quotes the native payment rail and always sets `fillOptions.nativeDispatchFee = 0` (the field remains in the on-chain struct; the native rail drew on a solver native balance nothing guaranteed). The estimate gains a `relayerFeeInSourceFeeToken` field for solver-side cost accounting.
- `ChainConfigService.getAssetBySymbol(chain, symbol)`: case-insensitive lookup into the per-chain asset table, which now ships curated mainnet deployments of ZARP, EURC, XSGD and TRYB (issuer-documented addresses, verified on-chain).


## 2.5.0

### Minor Changes

- `ORDER_PLACED` now returns the finalized canonical order without changing the order object passed to `execute()` or `placeOrder()`. Store and use `update.order` for the commitment, nonce, and session key; do not read those fields from the submitted object after placement.
- Source-side GET cancellation recovery now automatically clears stale recovery state and retries once with fresh proofs when Hyperbridge has pruned a required consensus update.

## 1.6.3

### Patch Changes

- Further improvements to intents v2

## 1.6.2

### Patch Changes

- Prevent stream from constructing hyperbridge finalized event when the request has been timeout on hyperbridge

## 1.6.1

### Patch Changes

- Improve substrate method

## 1.5.1

### Patch Changes

- Fix Hyperbridge delivery method

## 1.4.10

### Patch Changes

- Added storage query to check for refunded orders

## 1.4.7

### Patch Changes

- Added Storage resolvers

## 1.4.3

### Patch Changes

- fix issue with copy wasm file and also added support for monorepo root package.json

## 1.3.25

### Patch Changes

- Add latest statemachine query to the indexer

## 1.3.21

### Patch Changes

- Updated relayer fee in estimate fill order

## 1.3.20

### Patch Changes

- Fix Indexer stream in cancel order not yielding SOURCE FINALIZED

## 1.3.19

### Patch Changes

- Fix message submit in cancel order method

## 1.3.17

### Patch Changes

- Update polygon config

## 1.3.15

### Patch Changes

- Add retry logic for source proof in cancel order

## 1.3.14

### Patch Changes

- Minor fix

## 1.3.13

### Patch Changes

- Minor patch

## 1.3.12

### Patch Changes

- Bump sdk

## 1.3.11

### Patch Changes

- Yield source proof from cancel order method

## 1.3.10

### Patch Changes

- Implemented cancel order method in Intent class

## 1.3.4

### Patch Changes

- Added queryGetRequestStatus method to the SDK

## 1.3.0

### Minor Changes

- Expose the calldata for estimateGasForPost()
