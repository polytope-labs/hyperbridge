# 2026-09-22 — Mainnet gateway implementation and solver account deployed

The current `IntentGatewayV2` implementation, its two modules and a new `SolverAccount` are deployed
on all nine mainnet chains: ethereum (1), optimism (10), arbitrum (42161), base (8453), bsc (56),
gnosis (100), soneium (1868), polygon (137) and polkadot hub (420420419). The proxy address is
unchanged at `0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716`, and it still runs the old implementation.
Nothing about mainnet behaviour has changed yet.

New addresses, identical on all nine chains:

| Contract | Address |
|---|---|
| `IntentGatewayV2` implementation | `0x8E6c939E4915960eDa307c28F1d14840472F2648` |
| `IntrinsicModule` | `0x84478d65c814a9180030b2c8090dcCEf213eDF1D` |
| `ExtrinsicModule` | `0x07691e3B3F1D390dE0b83e232423038F9663CB45` |
| `SolverAccount` | `0xd5535d4DeB17F050e52B6efda2fDe00435f39279` |

## Upgrading a chain

The live implementation already understands the `Execute` action, so each chain moves with
`execute_on_gateway`, not `upgrade_gateway`. The call data is the same everywhere:

```
upgradeToAndCall(0x8E6c939E4915960eDa307c28F1d14840472F2648,
                 migrate(0x08BcC96ceC579Ce2eDb34E5C5d790e4A40c3e224))
```

`migrate` takes the proxy from `Initializable` version 2 to 3 and sets the owner, whose only powers
are `pause()` and `unpause()`. On mainnet slot 13 holds the removed `_paused` byte followed by
relayer `0xD15007f5fF5c8c5cCc77339A03a3EA60d1Ece854`, which is the layout `migrate` shifts, so the
relayer survives the upgrade and needs no follow-up `setRelayer`. Params are already
`surplusShareBps` 6000 and `protocolFeeBps` 5, so no `UpdateParams` is needed either.

## Escrow placed before the upgrade

`_orders` is keyed by leg index, where the live implementation keys it by token address, and
`migrate` does not re-key it. Escrow recorded before a chain is upgraded is therefore unreachable
afterwards: `_withdraw` reverts `UnknownOrder` when it reads zero. The `TRANSACTION_FEES` fee pot is
unaffected, since its sentinel key is the same number under both keyings.

Two consequences at cutover. Orders left open from before the upgrade can no longer be cancelled or
refunded — on 2026-09-22 that was 13 orders holding about 3.20 USDC and 999.50 cNGN on base and
0.9995 USDC on arbitrum. More importantly, a cross-chain order placed before the upgrade and filled
after it has its `RedeemEscrow` reverted on the source chain, and `EvmHost.dispatchIncoming` swallows
a failing `onAccept` and deletes the receipt, so the solver's redeem is consumed with no retry.
Upgrade a source chain when it has no orders in flight.

## Solvers must re-delegate

`SolverAccount` moved from `0x7cb55539…c88C`. An EOA still delegated to the old address runs the
previous validation logic, which derives the nonce key from the commitment and session key alone
rather than also from the op's calldata. The new account depends on the upgraded gateway. The SDK's
`chain.ts` points every mainnet chain at it (sdk 2.8.16, simplex 0.16.5), so solvers on those
releases can only bid once a chain's gateway is upgraded. The indexer's mainnet config lists only
the new account, so solvers still delegated to an old one are not tracked until they re-delegate.
