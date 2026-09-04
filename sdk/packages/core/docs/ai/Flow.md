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
   with `Unauthorized` unless the relayer equals `_relayer`. Only then is the first body byte read as
   a `RequestKind`. `onGetResponse` has the same two steps before touching the response.

So a delivery from anyone but the authorised relayer never decodes the body, never runs
`_authenticate`, and leaves no receipt. The authorised relayer submitting the same message later
takes the normal path. A gateway whose `_relayer` is zero refuses everything, since step 1 always
supplies a real address.

The address checked in step 3 is only as trustworthy as the contract in step 1, and that contract
is `_hostParams.handler`, which `HostManager.onAccept` can replace through a `SetHostParam`
request from Hyperbridge (`evm/src/core/HostManager.sol`, then `EvmHost.updateHostParams`). The
HostManager therefore runs the same relayer check before decoding any governance action, against a
relayer the host admin set with `setRelayer`; `testForgedHandlerSwapIsRefused` in
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
recipient the runtime chooses. Because the new manager's relayer gate applies to everything after
the swap, the host admin sets its relayer before the rotation is dispatched.

`setRelayer` has two callers. `_owner` calls it directly. The host reaches it when governance
sends `UpgradeContract` with `abi.encodeCall(setRelayer, (relayer))` as migration calldata:
`ERC1967Utils.upgradeToAndCall` delegatecalls that calldata into the new implementation with
`msg.sender` still the host from step 2, so the relayer is set in the same transaction as the
implementation swap. The upgrade message itself must already pass the relayer gate of the
implementation being replaced, which is why a fresh proxy cannot be armed that way:
`DeployIntentGateway.s.sol` calls `setRelayer` from the admin key immediately after deploying the
proxy, before anything can be escrowed into it.

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
anything but the callback selectors, so there is no equivalent of the gateway's host branch, and
none is needed: the token is not behind a proxy, so there is no upgrade transaction to arm it in.
