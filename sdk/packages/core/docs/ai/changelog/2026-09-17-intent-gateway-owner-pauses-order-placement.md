# 2026-09-17 — Intent gateway owner pauses order placement

`IntentGatewayV2` has an owner whose only power is `pause()` and `unpause()`. While paused,
`placeOrder` reverts `EnforcedPause()`; fills, cancellations and cross-chain settlement keep working,
so placed orders can still complete or be refunded. `paused()` reads the existing `_paused` flag
(slot 13), which is unset on every live proxy.

The owner and a pending owner live in the implementation at the ERC-7201 slot
`hyperbridge.storage.IntentGatewayV2.Ownership`, not in `IntentsBase`, so the modules and the
storage layout are unchanged. `owner()` and `pendingOwner()` read them. Transfers are two-step:
`transferOwnership(newOwner)`, callable by the owner or the host (zero withdraws a proposal), then
`acceptOwnership()` by the pending owner. Governance replaces the owner with an `Execute` carrying
`upgradeToAndCall(currentImplementation, transferOwnership(next))`.

`VERSION` is 4. `initialize(params, peerChains, relayer, owner)` sets the owner of a fresh proxy and
`migrate(address owner)`, host-only, sets it for a proxy at 2 or 3; both reject a zero owner.
`DeployIntentGateway.s.sol` reads `GATEWAY_OWNER` for new proxies, and
`intentGatewayUpgradeInitialization(gateway, owner)` builds `migrate(owner)` for upgrades.

The interface gains `owner`, `pendingOwner`, `transferOwnership`, `acceptOwnership`, `paused`,
`pause`, `unpause`, the `OwnershipTransferStarted`, `OwnershipTransferred`, `Paused` and `Unpaused`
events and the `EnforcedPause` error, and `migrate` takes the owner.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/flows/how-a-cross-chain-delivery-reaches-the-gateway-and-where-the.md`.
Gateway side: `evm/src/apps/IntentGatewayV2.sol`, `evm/script/DeployIntentGateway.s.sol`,
`evm/script/IntentGatewayScript.sol`, `evm/tests/foundry/IntentGatewayV2Test.sol`.
