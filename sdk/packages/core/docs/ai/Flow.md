# Flow

AI-maintained map of how code paths in `sdk/packages/core` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## What `IIntentGatewayV2` is, and what actually reads it

`contracts/apps/IntentGatewayV2.sol` is a declaration-only file: structs, errors, events, and
external function signatures. There is no implementation of `IIntentGatewayV2` in this package —
the gateway lives at `evm/src/apps/IntentGatewayV2.sol` and inherits its events and errors from
`evm/src/apps/intentsv2/IntentsBase.sol`.

Two consumers, and they use it very differently:

1. **`evm/src/apps/intentsv2/SolverAccount.sol`** imports it and reads exactly two things:
   `IIntentGatewayV2.select.selector` and `IIntentGatewayV2.fillOrder.selector`. It resolves via the
   `@hyperbridge/core/` remapping in `evm/remappings.txt`, which points at
   `node_modules/@hyperbridge/core/contracts/` — a workspace symlink, so an edit here is picked up
   by `forge build` in `evm/` with no publish step. Only the two function signatures matter to it.
2. **Integrators**, who read the file as the gateway's published ABI surface. Everything else in it
   — the events especially — exists for them alone.

That split is the whole reason the events drift: nothing in the repo compiles against them, so a
wrong signature is silent locally and only wrong for whoever depends on the package. The
declaration lists in this file and in `IntentsBase.sol` are kept identical; diffing them is the
only check that exists.

## How a cross-chain delivery reaches the gateway, and where the relayer gate sits

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

`setRelayer` (`evm/src/apps/intentsv2/ExtrinsicIntents.sol`) rotates the relayer; it,
`initialize` and `migrate` are the callers of `_setRelayer`, the only writer of `_relayer`. The
host reaches `setRelayer` when governance sends `UpgradeContract` with
`abi.encodeCall(setRelayer, (relayer))` as migration calldata: `ERC1967Utils.upgradeToAndCall`
delegatecalls that calldata into the new implementation with `msg.sender` still the host from
step 2, so `onlyHost` passes and the relayer is set in the same transaction as the implementation
swap. The upgrade message itself must pass the relayer gate of the implementation being replaced,
which is why an unset relayer gates nothing: a proxy from before the gate accepts every relayer
until the `migrate` upgrade arms it, and `testFreshProxyIsOpenUntilGovernanceArmsIt` plays both
halves on a proxy initialized with a zero relayer. `_owner` plays no part in any of
this. A fresh proxy is armed by its init data: `initialize` takes the relayer, writes it through
`_setRelayer`, and lands at `VERSION` (2) under `reinitializer`, emitting `RelayerUpdated` then
`Initialized(2)`. A proxy from before this implementation sits at 1 with an open gate until the
upgrade whose calldata is `abi.encodeCall(migrate, (relayer))`, host-only and under the same
`reinitializer(VERSION)`, arms it and takes it to 2. A `setRelayer` rotation leaves the version
alone. A revert from `version()` means an implementation from before the gate.
`testInitializeArmsTheGate` pins the fresh path, `testMigrateArmsAndBumpsTheVersion` and
`testMigrateRunsOnce` the migration, and the live-fork upgrade test reads 2 on the mainnet proxy
after it.

## The same gate on `HyperFungibleToken`, and how the BRIDGE token tightens it

Verified against `contracts/apps/HyperFungibleToken.sol` and `evm/src/apps/BridgeToken.sol`, and
exercised by the relayer tests in `evm/tests/foundry/HyperFungibleTokenTest.sol` and
`evm/tests/foundry/BridgeTokenTest.t.sol`.

Steps 1 and 2 above are identical; the token is just another `IApp`. Timeouts take a parallel route:
`HandlerV2.handlePostRequestTimeouts` calls `host.dispatchTimeOut(PostRequestTimeout(request,
_msgSender()), ...)`, and the host calls `onPostRequestTimeout` on the module.

3. `onAccept` and `onPostRequestTimeout` run `onlyHost`, `whenNotPaused`, then
   `_checkRelayer(incoming.relayer)`. Only after that is the source checked against
   `_supportedChains` or the body decoded, and only then does anything mint.

`_checkRelayer` is virtual. In `HyperFungibleToken` it reverts with `UnauthorizedRelayer` only when
`_relayer` is set and differs from the incoming relayer, so a token that never called `setRelayer`
accepts every relayer. `BridgeToken` overrides it to revert whenever the two differ, so with
`_relayer` unset nothing can mint; the deploy script therefore calls `setRelayer` before
`configure`, and before `configure` the token cannot be reached at all since `onlyHost` compares
against an unset `_host`.

`setRelayer` is `onlyOwner` in both. The host is not the owner and never calls the token with
anything but the callback selectors, so there is no equivalent of the gateway's host-only
`setRelayer`, and none is needed: the token is not behind a proxy, so there is no upgrade
transaction to arm it in.
