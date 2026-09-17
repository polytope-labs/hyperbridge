# 2026-09-17 — Intent gateway owner can pause the gateway

`IntentGatewayV2` has an owner whose only power is `pause()` and `unpause()`. While paused,
`placeOrder`, `fillOrder` and `onGetResponse` revert `EnforcedPause()`, and so does `onAccept` for any
request whose source is not Hyperbridge itself: escrow redemptions and refunds from peer gateways are
refused, governance deliveries still land. The checks run on the implementation before delegatecalling
a module. A refused delivery reverts, so the host deletes its receipt and the relayer can resubmit it
after `unpause`. `cancelOrder` is not paused. `paused()` reads the existing `_paused` flag (slot 13),
which is unset on every live proxy.

The owner comes from OpenZeppelin's `Ownable2StepUpgradeable`, inherited by the implementation only.
Its owner and pending owner sit at OpenZeppelin's ERC-7201 namespaced slots, so the modules and the
sequential storage layout are unchanged. `_checkOwner` is overridden to accept the host as well, so
governance can pause, resume, or propose a new owner (including after `renounceOwnership`) with an
`Execute` carrying `upgradeToAndCall(currentImplementation, call)`. Owner checks revert
`OwnableUnauthorizedAccount(address)`, and a zero owner `OwnableInvalidOwner(address)`.
`@openzeppelin/contracts-upgradeable` (`^5.6.1`, the version `@hyperbridge/core` already uses) is added
to `evm/package.json`.

`VERSION` stays 3. `initialize(params, peerChains, relayer, owner)` sets the owner of a fresh proxy
and `migrate(address owner)`, host-only, sets it for a proxy at 2. `DeployIntentGateway.s.sol` reads
`GATEWAY_OWNER` for new proxies, and `intentGatewayUpgradeInitialization(gateway, owner)` builds
`migrate(owner)` for upgrades.

The interface gains `owner`, `pendingOwner`, `transferOwnership`, `acceptOwnership`,
`renounceOwnership`, `paused`, `pause`, `unpause`, the `OwnershipTransferStarted`,
`OwnershipTransferred`, `Paused` and `Unpaused` events, and the `EnforcedPause`,
`OwnableUnauthorizedAccount` and `OwnableInvalidOwner` errors; `migrate` takes the owner.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/flows/how-a-cross-chain-delivery-reaches-the-gateway-and-where-the.md`.
Gateway side: `evm/src/apps/IntentGatewayV2.sol`, `evm/package.json`, `evm/pnpm-lock.yaml`,
`evm/script/DeployIntentGateway.s.sol`, `evm/script/IntentGatewayScript.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`.
