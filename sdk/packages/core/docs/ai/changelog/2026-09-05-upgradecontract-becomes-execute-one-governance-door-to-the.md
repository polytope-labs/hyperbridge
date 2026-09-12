# 2026-09-05 — `UpgradeContract` becomes `Execute`, one governance door to the gateway's host-only functions

`RequestKind.Execute` (discriminator 5, the old `UpgradeContract` slot) delegatecalls the proxy's
current implementation with `body[1:]` as calldata, the host still `msg.sender`. `upgradeToAndCall`
is now a host-only function on `ExtrinsicIntents` wrapping `ERC1967Utils.upgradeToAndCall`, so an
upgrade is `Execute` carrying that call, and a relayer rotation is `Execute` carrying `setRelayer`
with no implementation change. Reverts inside the call bubble out. The `(address, bytes)` body of
the old action selects no function on this implementation and reverts, and the live
implementation still reads it as `UpgradeContract`, so the pallet keeps `upgrade_gateway` for the
one upgrade that installs this code on a chain and gains `execute_on_gateway(data)` for
everything after, weighed as `upgrade_gateway`. The interface declares `upgradeToAndCall`.

Tests: `_upgradeRequest` builds `Execute` + `upgradeToAndCall`, so the existing upgrade tests run
through the new path; new `testExecuteRotatesRelayerWithoutUpgrade`,
`testExecuteRejectsNonHyperbridgeSource`, `testExecuteBubblesReverts`,
`testLegacyUpgradeBodyIsRefused`, `testUpgradeToAndCallRejectsEveryoneButHost`; the live-fork
test migrates the mainnet proxy with the legacy body and then rotates it through `Execute`.
Pallet tests pin the `Execute` encoding and the new extrinsic.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/intentsv2/IntentsBase.sol`,
`evm/src/apps/intentsv2/ExtrinsicIntents.sol`, `evm/src/apps/IntentGatewayV2.sol`,
`evm/script/DeployIntentGateway.s.sol`, `evm/script/DeployIntentGatewayImpl.s.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`, `modules/pallets/intents-coprocessor/src/types.rs`,
`modules/pallets/intents-coprocessor/src/lib.rs`, `modules/pallets/intents-coprocessor/src/tests.rs`.
