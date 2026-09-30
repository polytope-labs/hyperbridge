# 2026-08-27 — `FillOptions` carries a `validUntil`, and `fillOrder` has two shapes

`FillOptions` gained `validUntil` (a block number; `0n` means unbounded), enforced by `fillOrder`, which now reverts
`FillExpired` past it. Adding a field changes the enclosing function's selector, so `fillOrder` exists in two
incompatible shapes — `0x5cfb1ea5` (v1) and `0xa5470064` (v2) — and gateways upgrade per chain, so both are on the
wire at once.

New `protocols/intents/fillOrderCodec.ts` owns that: `getFillOptionsVersion` reads the gateway's ERC-1967
implementation slot and matches the address against a set of known pre-`validUntil` implementations, defaulting to
v2; `encodeFillOrder` emits the matching shape and `decodeFillOrder` reads either. There is no version getter on
the contract — EIP-1967 has no version field either, and the implementation address is the value the proxy already
updates on upgrade. `GasEstimator` encodes through it so estimates do not revert on a missing function; `BidManager` and
`phantom-aggregation.extractFillData` decode through it so bids built against an older gateway are still priced in
rather than dropped.

Why the field exists: a solver bidding through the coprocessor signs this calldata and then has no further say in
when it is used. The order's `deadline` is placer-chosen with no ceiling, retracting the bid on Hyperbridge does not
reach the destination chain, and the placer holds the session key — so a signed bid stayed executable indefinitely
and was taken up only once the rate had moved against the solver. `validUntil` rides in the calldata, which
`userOpHash` already covers, so it is tamper-proof without touching the signature format.

`fillOrder` also no longer calls `IIntentPriceOracle.recordSpread`. It passed `order.inputs` and `options.outputs`
to a stateful oracle verbatim, and neither is validated against anything that costs the caller money — the escrow
lives on the source chain and is never consulted on the destination side. `Params.priceOracle` is left in place
because removing a field from a storage struct behind an upgradeable proxy shifts the layout.

Found by the scheduled IntentGateway/Simplex security audit.

Files: `src/protocols/intents/fillOrderCodec.ts` (new), `src/protocols/intents/GasEstimator.ts`,
`src/protocols/intents/BidManager.ts`, `src/protocols/intents/phantom-aggregation.ts`,
`src/protocols/intents/index.ts`, `src/abis/IntentGatewayV2.ts`, `src/types/index.ts`,
`src/tests/fillOrderCodec.test.ts` (new), `src/tests/phantomAggregation.test.ts`, plus
`evm/src/apps/IntentGatewayV2.sol`, `evm/src/apps/intentsv2/IntentsBase.sol` and
`sdk/packages/core/contracts/apps/IntentGatewayV2.sol`.
