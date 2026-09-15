# How a cross-chain delivery reaches the gateway, and where the relayer gate sits

Verified against `evm/src/core/HandlerV2.sol`, `evm/src/core/EvmHost.sol` and
`evm/src/apps/intentsv2/ExtrinsicIntents.sol`, and exercised by
`testRejectedDeliveryStaysRetryableThroughHost` in `evm/tests/foundry/IntentGatewayV2Test.sol`.

1. A relayer calls `HandlerV2.handlePostRequests` (or `handleGetResponses`). After proof
   verification the handler calls `host.dispatchIncoming(request, _msgSender())`. `_msgSender()` is
   plain `msg.sender`; the handler has no trusted forwarder.
2. `EvmHost.dispatchIncoming` (restricted to the handler) writes a receipt for the request
   commitment, then low-level calls the module with `IApp.onAccept(IncomingPostRequest(request,
   relayer))`. If that call fails the host deletes the receipt and returns without reverting, so the
   rest of the batch proceeds and the message stays deliverable.
3. `ExtrinsicIntents.onAccept` runs `onlyHost`, then `_checkRelayer(incoming.relayer)`, which reverts
   with `Unauthorized` when a relayer is set and the delivery is from anyone else. Only then is the
   first body byte read as a `RequestKind`. `onGetResponse` has the same two steps before touching
   the response.

So a delivery from anyone but the authorised relayer never decodes the body, never runs
`_authenticate`, and leaves no receipt. The authorised relayer submitting the same message later
takes the normal path. A gateway whose `_relayer` is zero accepts every relayer: that is the state
a fresh proxy is in until governance arms it, below.

The address checked in step 3 is only as trustworthy as the contract in step 1, and that contract
is `_hostParams.handler`, which `HostManager.onAccept` can replace through a `SetHostParam`
request from Hyperbridge (`evm/src/core/HostManager.sol`, then `EvmHost.updateHostParams`). The
HostManager therefore runs the same relayer check before decoding any governance action, against
its admin: the account named in its constructor, which is also the only one allowed to bind the
host with `init` when the host was not known at construction. `testForgedHandlerSwapIsRefused` in
`evm/tests/foundry/HostManagerTest.sol` plays the swap through the real host and shows it refused.
The HostManager sees no user traffic, so this leaves ordinary relaying open.

Replacing the HostManager itself follows the same route. `update_host_params` on Hyperbridge
(`modules/pallets/host-executive/src/lib.rs`) reads the stored params, remembers the current
manager, applies the update, and dispatches the encoded result addressed to the manager it
remembered. That manager's `onAccept` calls `EvmHost.updateHostParams`, which is restricted to
`_hostParams.hostManager`, so the host swaps to the new manager only because the call arrived
through the old one. Hyperbridge records the new manager as soon as the request is dispatched, and
every later `update_host_params` and `withdraw` is addressed to it. The two rotation tests in
`evm/tests/foundry/HostManagerTest.sol` play both addressings through the real host, and
`test_manager_rotation_is_addressed_to_the_current_manager` in the pallet testsuite pins the
recipient the runtime chooses. The new manager is constructed with the governance relayer as its
admin, so its gate is armed before the rotation is dispatched.

Rotating the relayer itself is the `SetAdmin` action. `set_host_manager_admin` on Hyperbridge
reads the manager on record, refuses a zero admin, and dispatches `[2] ++ abi.encode(admin)`
(`encode_set_admin` in `evm/rust/src/host_params.rs`) addressed to that manager. On delivery
`onAccept` runs the relayer gate and the Hyperbridge-source check as for any action, then decodes
the address, refuses zero, emits `AdminUpdated` and stores it; the outgoing admin delivers it and
is locked out from the next message on. `testSetAdminRotates` plays it in Solidity,
`test_host_manager_set_admin` in `evm/tests/rust/src/tests/host_manager.rs` delivers the pallet's
encoding to the compiled contract, and `test_set_host_manager_admin_is_addressed_to_the_manager`
pins the recipient and body the runtime dispatches.

`setRelayer` (`evm/src/apps/intentsv2/ExtrinsicIntents.sol`) rotates the relayer; it and
`initialize` are the callers of `_setRelayer` (`IntentsBase.sol`), the only writer of `_relayer`.
Since the module split (#1262) the gateway proxy's implementation delegatecalls `onAccept` and
`onGetResponse` to the `ExtrinsicModule`, so `ExtrinsicIntents.onAccept` runs in the proxy's
storage with the host still `msg.sender`. Governance reaches every host-only function through one
action, `Execute` (discriminator 5): `onAccept` delegatecalls the module's own address (`__self`)
with `body[1:]` as calldata, another hop that keeps the host as `msg.sender` so `onlyHost` passes.
`setRelayer(next)` as that calldata is a rotation, `upgradeToAndCall(newImpl, initData)` is an
upgrade, and inside the latter `ERC1967Utils.upgradeToAndCall` delegatecalls `initData` into the
new implementation, still with the host as `msg.sender`, which is how `migrate()` bumps the
version in the same transaction as the swap. `setRelayer` and `upgradeToAndCall` exist only on the
module, not on the implementation, so init data cannot rotate the relayer: an upgrade and a
rotation are two `Execute` messages (`testUpgradeThenRotateAreTwoExecutes`). A revert anywhere
inside bubbles out of `onAccept`, so the host records the message undelivered. The pallet's
`execute_on_gateway(data)` prepends the discriminator to `data`; `upgrade_gateway` sends the older
`UpgradeContract` body under the same discriminator, which only a pre-`Execute` implementation
reads (on this one it selects no function and reverts, `testLegacyUpgradeBodyIsRefused`). The
message itself must pass the relayer gate of the implementation it reaches, which is why an unset
relayer gates nothing: `testFreshProxyIsOpenUntilGovernanceArmsIt` plays both halves on a proxy
initialized with a zero relayer. `testExecuteRotatesRelayerWithoutUpgrade` and the live-fork test's
rotation after the migration pin the `Execute` path. The implementation no longer has an `_owner`;
it was a placeholder with nothing to do and left with the module split. A fresh proxy is armed by
its init data: `initialize` takes the relayer, writes it through `_setRelayer`, and lands at
`VERSION` (3) under `reinitializer`, emitting `RelayerUpdated` then `Initialized(3)`; it is refused
on any proxy already at a version. A proxy on the previous implementation sits at 2, armed, until
the upgrade whose init data is `abi.encodeCall(migrate, ())`, host-only and under the same
`reinitializer(VERSION)`, takes it to 3; that is the only way up for it, since `initialize` is
refused on anything but a bare proxy. `migrate()` changes nothing else. A `setRelayer` rotation
leaves the version alone. A revert from `version()` means an implementation from before the gate.
`testInitializeArmsTheGate` pins the fresh path, `testMigrateBumpsTheVersion` and
`testMigrateRunsOnce` the migration, `testUpgradeFromVersionTwoWithMigrate`
(`evm/tests/foundry/IntentGatewayModulesTest.sol`) the release's own upgrade from 2, and the
live-fork test reads 2 on the mainnet proxy, upgrades it with `migrate()` to 3, and shows it
refuses `initialize` and a second `migrate`.
