# Gateway release 4 and the official ERC-4337 and Permit2 interfaces

IntentGatewayV2 now lands proxies at `VERSION` 4. Release 4 is the implementation that binds solver
selection to the EntryPoint v0.9 UserOperation. Storage is the same as at 3.

`migrate()` takes a proxy from 3 to 4 and writes nothing else. It takes no arguments and runs only
on a proxy exactly one version behind. The v2 path, which moved the relayer slot and set the owner,
is gone, because every live proxy is already at 3. `IIntentGatewayV2` in `@hyperbridge/core` no
longer declares `migrate`.

Governance upgrades a gateway with `execute_on_gateway` carrying
`upgradeToAndCall(newImpl, migrate())`. `intentGatewayUpgradeInitialization` in
`evm/script/IntentGatewayScript.sol` builds that init data for a proxy at 3, and empty init data for
one already at 4. Upgrades no longer read `GATEWAY_OWNER`.

The gateway also exposes `entrypoint()`, the EntryPoint v0.9 address whose
`getCurrentUserOpHash()` gates a selected fill.

`fillOrder` checks each leg's shape just before filling that leg, not all legs up front. A bad later
leg still reverts the whole fill.

The SDK's `SUPPORTED_INTENTS_VERSION` is 4. An SDK on 3 refuses a gateway once it is upgraded, and
this SDK refuses one that is not, so the SDK release and the gateway upgrades ship together. The SDK
releases as `@hyperbridge/sdk` 3.0.0, a major version because it refuses every release-3 gateway.

The SDK's mainnet chain config points `SolverAccount` at
`0xaAd062555800a97Af062795189e32a3CBd045612` on Ethereum, BSC, Arbitrum, Base, Polygon and
Polkadot Hub. On the first five, `EntryPoint` moves to v0.9
(`0x433709009B8330FDa32311DF1C2AFA402eD8D009`), the EntryPoint the new account validates against.
Polkadot Hub has no EntryPoint deployed. Optimism, Gnosis and Soneium keep their previous values,
since their gateways are not upgraded. Simplex reads both addresses from this config.

The indexer lists the new account first and `0x77c3394CA5881A74f18139AC87D0c11F8Faa90cC` second on
its five mainnet chains, so solvers keep counting while they re-delegate.
`0xd5535d4DeB17F050e52B6efda2fDe00435f39279` is no longer listed.

`evm/` now takes interfaces from their official packages instead of declaring its own:

- `@account-abstraction/contracts@0.9.0-rc.1` for the EntryPoint (`IEntryPoint`, `IStakeManager`).
- `@uniswap/permit2` from `github:Uniswap/permit2#cc56ad0` for `ISignatureTransfer` and
  `IAllowanceTransfer`. Permit2's Solidity is not published to npm.

SimplexPaymaster keeps OpenZeppelin's `IEntryPoint`, which its `PaymasterERC20` base requires. Its
v0.8 calls are explicit casts on the `ENTRYPOINT_V08` address, and `getDepositInfo`, which
OpenZeppelin's interface lacks, goes through the package's `IStakeManager`.

OpenZeppelin's account bases and the package each declare a `PackedUserOperation` struct, and
Solidity does not convert between them. Tests that send one op to both convert it at the EntryPoint
call with `toEntryPointOp` and `toEntryPointOps` from `evm/tests/foundry/EntryPointOps.sol`.
