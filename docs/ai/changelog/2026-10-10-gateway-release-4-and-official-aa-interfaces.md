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
this SDK refuses one that is not, so the SDK release and the gateway upgrades ship together.

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
