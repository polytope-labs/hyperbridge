# 2026-09-17 — Intent gateway owner can pause the gateway

`IntentGatewayV2` has an owner whose only power is `pause()` and `unpause()`. While paused,
`placeOrder`, `fillOrder` and `onGetResponse` revert `EnforcedPause()` through `whenNotPaused`, and so
does `onAccept` for any request whose source is not Hyperbridge itself: escrow redemptions and refunds
from peer gateways are refused, governance deliveries still land. The checks run on the implementation
before delegatecalling a module. A refused delivery reverts, so the host deletes its receipt and the
relayer can resubmit it after `unpause`. `cancelOrder` is not paused.

The pause is OpenZeppelin's `PausableUpgradeable`, whose flag sits at its ERC-7201 namespaced slot.
`pause()` while paused reverts `EnforcedPause()` and `unpause()` while not paused `ExpectedPause()`.
`IntentsBase`'s unused `bool _paused` (slot 13 offset 0, unset on every live proxy) is removed, so
`_relayer` moves from slot 13 offset 1 to offset 0. `migrate` moves it on existing proxies by shifting
slot 13 right one byte, which also drops the old flag. The upgrade from 2 must carry `migrate`: without
it the relayer gate reads a wrong address and refuses every delivery.

The owner comes from OpenZeppelin's `Ownable2StepUpgradeable`, inherited by the implementation only.
Its owner and pending owner sit at OpenZeppelin's ERC-7201 namespaced slots, so the modules and the
sequential storage layout are unchanged. `_checkOwner` is overridden to accept the host as well, so
governance can pause, resume, or propose a new owner (including after `renounceOwnership`) with an
`Execute` carrying `upgradeToAndCall(currentImplementation, call)`. Owner checks revert
`OwnableUnauthorizedAccount(address)`, and a zero owner `OwnableInvalidOwner(address)`.
`@openzeppelin/contracts-upgradeable` (`^5.6.1`, the version `@hyperbridge/core` already uses) is added
to `evm/package.json`.

`VERSION` stays 3. `initialize` takes one `InitParams` struct (`params`, `peerChains`, `relayer`,
`owner`), declared next to `Params`, and sets the owner of a fresh proxy. `migrate(address owner)`,
host-only, moves the relayer and sets the owner for a proxy at 2. `DeployIntentGateway.s.sol` reads
`GATEWAY_OWNER` for new proxies, and `intentGatewayUpgradeInitialization(gateway, owner)` builds
`migrate(owner)` for upgrades.

The core contracts gain `InitParams`. The interface gains `owner`, `pendingOwner`, `transferOwnership`, `acceptOwnership`,
`renounceOwnership`, `paused`, `pause`, `unpause`, the `OwnershipTransferStarted`,
`OwnershipTransferred`, `Paused` and `Unpaused` events, and the `EnforcedPause`, `ExpectedPause`,
`OwnableUnauthorizedAccount` and `OwnableInvalidOwner` errors; `migrate` takes the owner.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/flows/how-a-cross-chain-delivery-reaches-the-gateway-and-where-the.md`.
Gateway side: `evm/src/apps/IntentGatewayV2.sol`, `evm/package.json`, `evm/pnpm-lock.yaml`,
`evm/script/DeployIntentGateway.s.sol`, `evm/script/IntentGatewayScript.sol`,
`evm/src/apps/intentsv2/IntentsBase.sol`, `evm/tests/foundry/IntentGatewayV2Test.sol`,
`evm/tests/foundry/IntentGatewayModulesTest.sol`, and every test that initializes a gateway.
