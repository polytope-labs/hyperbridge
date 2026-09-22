# 2026-09-22 — One pair per intent order

`IntentGatewayV2.placeOrder` now requires every leg of an order to trade the same pair. All of
`order.inputs` must name one token and all of `order.output.assets` must name one token, or
placement reverts with `InvalidInput`. An order is therefore a single pair quoted at one or more
prices — a ladder — rather than a basket.

Leg-level accounting is unchanged. `_orders`, `_partialFills` and `_protocolFees` stay keyed by
`(commitment, leg index)`, so each leg still carries its own escrow, fill progress and held
protocol fee, different solvers can fill different legs, and a cancel refunds each leg's own
remainder.

## What the rule replaces

The upper-12-bytes check on a token now runs once, on leg 0, for the inputs and for the outputs.
The remaining legs must equal leg 0 byte for byte, which also rules out a token named in two forms:
`T` and `T | 1 << 255` are no longer two legs of one order.

An order with predispatch calldata must now be single-leg. Its escrow is swept from the
`CallDispatcher` and measured per input token, so two of its legs never could share a token. Now
that every leg holds the same token, that is simply a leg count, and a length check replaces the
pairwise comparison that used to express it.

## Unchanged

The fill path is untouched. `_validateLegs` still checks every leg's tokens against the solver's
quote and still rejects an output token with its upper 12 bytes set, because the destination chain
of a cross-chain order never sees `placeOrder`. `_execute` still sweeps each output token at its
first leg only.

## Files

`evm/src/apps/IntentGatewayV2.sol`, `evm/tests/foundry/IntentGatewayV2MultiLegTest.sol`,
`evm/tests/foundry/IntrinsicIntentsReentrancyTest.sol`, `evm/tests/foundry/IntentGatewayV2Test.sol`,
`sdk/packages/core/contracts/apps/IntentGatewayV2.sol` (the `placeOrder` doc comment) and
`docs/content/developers/evm/intent-gateway/overview.mdx`.

Nothing in the ABI changes. The rule reuses `InvalidInput`, so the gateway ABIs carried by the SDK,
Simplex and the indexer stay as they are.
