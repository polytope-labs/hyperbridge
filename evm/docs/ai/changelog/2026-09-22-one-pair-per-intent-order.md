# 2026-09-22 — One pair per intent order

`IntentGatewayV2.placeOrder` accepts an order only if every leg trades the same pair. All of
`order.inputs` must name one token and all of `order.output.assets` must name one token, or
placement reverts with `InvalidInput`. An order is a single pair quoted at one or more prices — a
ladder — rather than a basket.

A token is the address in the low 20 bytes of its `bytes32`, and the upper 12 must be zero. Legs
have to match byte for byte, so one address written two ways — `T` and `T | 1 << 255` — counts as
two tokens and cannot appear in one order.

An order carrying both predispatch calldata and predispatch assets must be single-leg. Its escrow
is swept from the `CallDispatcher` and measured per input token, and every leg now holds the same
token. Predispatch only runs when both fields are set; calldata without assets is ignored, as
before.

Leg-level accounting is unchanged. `_orders`, `_partialFills` and `_protocolFees` stay keyed by
`(commitment, leg index)`. Each leg carries its own escrow, fill progress and held protocol fee,
different solvers can fill different legs, and a cancel refunds each leg's own remainder.

The fill path is unchanged. `_validateLegs` checks every leg's tokens against the solver's quote,
and rejects an output token with its upper 12 bytes set, because the destination chain of a
cross-chain order never sees `placeOrder`. `_execute` sweeps each output token at its first leg
only.

Nothing in the ABI changes. The rule reuses `InvalidInput`, so the gateway ABIs carried by the SDK,
Simplex and the indexer stay as they are.

## Rollout

The rule takes effect on a chain once governance upgrades that chain's implementation, so for a
while some chains will accept an order shape that others refuse. `VERSION` stays 3, since no proxy
needs re-initialising, which means `version()` does not tell an upgraded gateway from one still
accepting baskets. Orders already placed are unaffected: their commitments, escrow and pending
messages run through the same code paths as before.
