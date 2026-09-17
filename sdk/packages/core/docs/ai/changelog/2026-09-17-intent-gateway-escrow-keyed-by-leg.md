# 2026-09-17 — Intent gateway escrow keyed by leg

An order is a list of legs again: leg `i` sells `order.inputs[i]` for `order.output.assets[i]`.
`placeOrder` requires both arrays non-empty and of equal length with every output amount non-zero,
and legs may repeat tokens, e.g. one pair offered at several prices. An order with predispatch
calldata may not repeat an input token: its escrow is swept and measured per token, and the
dispatcher does not check a token's `transfer` return value.

A token is the address in the low 20 bytes of its `bytes32`, and its upper 12 bytes must be zero:
`placeOrder` rejects such an input token and both fill paths such an output token, with
`InvalidInput`. The repeated-token checks compare the full `bytes32` while every transfer reads only
the address, so `T` and `T | 1 << 255` would pass as two tokens. With a token whose `transfer` returns
false instead of reverting, two such predispatch legs would each credit the one sweep that landed, and
a cancel would pay the second copy out of other orders' escrow in that token
(`testPlaceOrder_PredispatchRejectsAliasedInputToken`). Output tokens are checked at fill, on the
chain they belong to, rather than at placement.

`_orders`, `_partialFills` and `_protocolFees` are keyed by `(commitment, leg index)` instead of by
token, in the same storage slots (9, 11 and 14). Each leg's escrow, fill progress and held protocol
fee are its own, so a completing leg releases only its escrow (SRLabs S3-2), a repeated output token
cannot mark another leg complete (S2-15), and a cancel refunds every leg's remainder. The relayer
fee pot stays at `_orders[commitment][TRANSACTION_FEES]`, now a `uint256` key in the same slot.
Source-side cancellation proves `_partialFills[commitment][i]` for each leg, so legs sharing an
output token get distinct proof keys. Fills with output calldata sweep each output token from the
dispatcher once.

Getter signatures change: `_orders(bytes32,uint256)`, `_partialFills(bytes32,uint256)` and
`_protocolFees(bytes32,uint256)`. The SDK refund check, Simplex's source-escrow and partial-fill
reads, and the indexer, SDK and Simplex ABIs pass the leg index.

Rollout: an implementation keyed by token and one keyed by leg read the same slots differently, and
the source chain computes the proof key the destination stores under. Upgrade only once no order
placed under the old implementation holds escrow, fees or pending messages on any chain, and keep
placement stopped on every chain until all of them run this implementation.

Files: `contracts/apps/IntentGatewayV2.sol`. Gateway side: `evm/src/apps/IntentGatewayV2.sol`,
`evm/src/apps/intentsv2/IntentsBase.sol`, `evm/src/apps/intentsv2/IntrinsicIntents.sol`,
`evm/src/apps/intentsv2/ExtrinsicIntents.sol`, `evm/tests/foundry/IntentGatewayV2MultiLegTest.sol`.
