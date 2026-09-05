# ChangeLog

AI-maintained log of code changes in `sdk/packages/core`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/core/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-09-05 — `UpgradeContract` becomes `Execute`, one governance door to the gateway's host-only functions

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

## 2026-09-05 — Gateway `initialize` refused on any proxy already at a version

`initialize` carries an `onlyFresh` modifier that reverts with `InvalidInitialization` unless the
`Initializable` version is 0. Without it, an upgrade that installed this implementation on a
version-1 proxy without running `migrate` would leave `initialize`, which has no caller
restriction, open to anyone until governance caught up. Now the host-only `migrate` is the only
way up for such a proxy. `testInitializeRefusedOnLegacyProxy` plays it. The interface NatSpec for
`migrate` says so.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/IntentGatewayV2.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`.

## 2026-09-05 — Relayer gate lifted out of `HyperFungibleToken` into `BridgeToken`

`HyperFungibleToken` no longer carries `_relayer`, `relayer()`, `setRelayer`, `RelayerUpdated`,
`UnauthorizedRelayer` or the `_checkRelayer` hook; `onAccept` and `onPostRequestTimeout` are
`public virtual` so a token can gate deliveries before calling `super`. `BridgeToken` owns all of
that now: it overrides both callbacks with `onlyHost`, then `_checkRelayer`, then `super`, and
`_checkRelayer` fails closed as before. The base-token relayer tests in
`HyperFungibleTokenTest.sol` are gone with the feature; `BridgeTokenTest.t.sol` references the
error and event on `BridgeToken`. Storage of non-upgradeable `HyperFungibleToken` deployments
shifts by one slot, which only matters for a contract that reads it by slot.

Files: `contracts/apps/HyperFungibleToken.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/BridgeToken.sol`,
`evm/tests/foundry/HyperFungibleTokenTest.sol`, `evm/tests/foundry/BridgeTokenTest.t.sol`.

## 2026-09-05 — Gateway `initialize` takes the relayer and lands at version 2; `migrate` for older proxies

`IntentGatewayV2.initialize(Params, bytes[] peerChains, address relayer)` now arms the relayer
gate from the init data and runs under `reinitializer(VERSION)` with `VERSION = 2`, so a fresh
proxy comes out armed and at the version of the code it runs. `migrate(address relayer)`, host-only
and under the same `reinitializer(VERSION)`, is for proxies deployed before this implementation:
it arms them and takes them from 1 to 2, and reverts on a proxy `initialize` already took there.
`setRelayer` stays a plain host-only rotation that leaves the version alone; all three write
through `_setRelayer` in `ExtrinsicIntents`. The next implementation that needs a migration bumps
`VERSION` once. The interface documents `migrate` and `version` accordingly.

`DeployIntentGateway.s.sol` always deploys the implementation and the solver account, deploys the
proxy only where `INTENT_GATEWAY_V2` is absent from the chain's config, reads the relayer from
`GATEWAY_RELAYER` for the init data, and records `INTENT_GATEWAY_V2_IMPL`. The relayer is now part
of what fixes a new proxy's address, as the implementation address already was.

The reinitializer cost more than the 71 bytes of EIP-170 headroom, and every gateway getter and
event has a consumer in `sdk`, `simplex` or the indexer, so the room came from deduplicating
internal code with no behaviour change: `_sendValue` in `IntentsBase` for the native
send-and-check (the same-chain fill loop keeps its inline copy, being at the via-ir stack limit),
`_splitSurplus` moved to `IntentsBase` and used by the cross-chain fill, `_withdrawalBody` and
`_postToSource` in `ExtrinsicIntents` for the escrow messages, and `placeOrder` reusing its
`feeToken` read and hashing the order once.

Tests: every `initialize` call gains the relayer argument, `address(0)` outside `setUp` so those
gateways stay open as before; `testInitializeArmsTheGate` pins the events and version; the
`migrate` tests run on a proxy written back to version 1 through the `Initializable` slot, since
this implementation cannot produce one; the live mainnet-fork upgrade migrates the real one.
`HostManager.onAccept` gained NatSpec.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/IntentGatewayV2.sol`,
`evm/src/apps/intentsv2/ExtrinsicIntents.sol`, `evm/src/apps/intentsv2/IntrinsicIntents.sol`,
`evm/src/apps/intentsv2/IntentsBase.sol`, `evm/src/core/HostManager.sol`,
`evm/script/DeployIntentGateway.s.sol`, `evm/tests/foundry/IntentGatewayV2Test.sol`,
`evm/tests/foundry/IntentGatewayV2SameChainTest.sol`,
`evm/tests/foundry/IntrinsicIntentsReentrancyTest.sol`,
`evm/tests/foundry/account/SolverAccountTest.sol`.

## 2026-09-05 — `HostManager` admin is the governance relayer; gateway `setRelayer` is host-only

`HostManager` no longer has a separate relayer. Its `admin` survives initialization and is the only
relayer whose `onAccept` deliveries are accepted, so `_relayer`, `relayer()` and `setRelayer` are
gone. `setIsmpHost` is now `init`: admin-only, one-shot, and unnecessary when the host is passed to
the constructor. A new `SetAdmin` action (variant `2`, body `abi.encode(address)`) rotates the admin
through governance, delivered by the outgoing admin like every other message; zero is refused there
and in the constructor, since a manager with no admin could never be reached again.
`pallet-ismp-host-executive` gains `set_host_manager_admin`, which dispatches that action to the
manager on record, and `ismp-abi` gains `encode_set_admin`. `evm/rust/abi/HostManager.json` was
regenerated; it had not been since before the relayer gate.

On the gateway, `setRelayer` moved from `IntentGatewayV2` to `ExtrinsicIntents` and is `onlyHost`,
so `_owner` can no longer rotate the relayer; the host reaches it only as `UpgradeContract`
migration calldata, and nothing else writes the relayer. `initialize` is unchanged, so a fresh
proxy starts with no relayer, and an unset relayer now gates nothing: the governance upgrade that
arms it has to be delivered first. `setRelayer(address(0))` reopens the gate rather than closing
it. `DeployIntentGateway.s.sol` no longer reads `GATEWAY_RELAYER` or calls `setRelayer`.
`DeployIsmp.s.sol` constructs the host before the manager and binds the manager at construction,
with `GOVERNANCE_RELAYER` as its admin; `DeployHostManager.s.sol` does the same for a replacement
manager. Both contracts expose `relayer()`, and the gateway exposes `version()`, the
`Initializable` version the proxy has reached (1 after `initialize`, higher only after a
`reinitializer` migration), so tooling can tell which relayer a deployment accepts and whether it
has the gate at all (a revert means it predates it). The gateway's `_relayer` and `_instances`
became internal to pay for the getters under EIP-170; `instance(bytes)` already covered the
latter. The interface in this package declares the two getters and updates its `setRelayer`
NatSpec.

Files: `contracts/apps/IntentGatewayV2.sol`, `package.json`, `docs/ai/ChangeLog.md`,
`docs/ai/Decisions.md`, `docs/ai/Flow.md`. Outside the package: `evm/src/core/HostManager.sol`,
`evm/src/apps/IntentGatewayV2.sol`, `evm/src/apps/intentsv2/ExtrinsicIntents.sol`,
`evm/script/DeployIsmp.s.sol`, `evm/script/DeployHostManager.s.sol`,
`evm/script/DeployIntentGateway.s.sol`, `evm/tron/migrations/2_deploy_ismp.js`,
`evm/tron/README.md`, `evm/rust/src/host_params.rs`, `evm/rust/abi/HostManager.json`,
`evm/tests/foundry/HostManagerTest.sol`, `evm/tests/foundry/IntentGatewayV2Test.sol`,
`evm/tests/foundry/IntentGatewayV2SameChainTest.sol`, the foundry test setups that construct a
`HostManager` or initialize a gateway, `evm/tests/rust/src/tests/utils.rs`,
`evm/tests/rust/src/tests/host_manager.rs`, `modules/pallets/host-executive/src/lib.rs`,
`modules/pallets/testsuite/src/tests/pallet_ismp_host_executive.rs`.

## 2026-09-04 — Fresh gateway deployments arm the relayer in the deploy script

`DeployIntentGateway.s.sol` deployed the gated gateway with `_relayer` unset, so a proxy on a new
chain refused every delivery, including the `upgrade_gateway` message that could have armed it;
only the owner key could, and nothing called it. The script now reads `GATEWAY_RELAYER`, requires
the deploy key to be the admin (the only caller of `setRelayer`), calls `setRelayer` right after
the proxy is deployed, and asserts the relayer afterwards. `initialize` is unchanged so the
deterministic proxy address is unchanged. The gateway constructor now rejects a zero owner, since
a mis-set `ADMIN` would leave no key able to arm a fresh proxy; runtime size is unaffected. No
file in this package changed.

Files: `evm/script/DeployIntentGateway.s.sol`, `evm/src/apps/IntentGatewayV2.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`, `docs/ai/Flow.md`.

## 2026-09-03 — Host manager rotation addressed to the manager the host still trusts

`pallet-ismp-host-executive::update_host_params` applied the update before reading the request
recipient, so a rotation from one HostManager to another was addressed to the new one. The host
accepts `updateHostParams` only from its current manager, so the delivery reverted, Hyperbridge
recorded the new manager anyway, and every later host update and withdrawal for that chain was
sent to a contract the host did not trust. The recipient is now captured before the update; the
payload still installs the new manager. This is the path the HostManager relayer gate is rolled
out through. No file in this package changed.

Files: `modules/pallets/host-executive/src/lib.rs`,
`modules/pallets/testsuite/src/tests/pallet_ismp_host_executive.rs`,
`evm/tests/foundry/HostManagerTest.sol`, `docs/ai/Flow.md`.

## 2026-09-03 — Governance deliveries to `HostManager` gated on the same relayer

Closes the route around the app-level gates: `HostManager.onAccept` accepted `SetHostParam` from
any relayer, and that request can replace the host's handler, the contract every app trusts to
report the relayer address. `HostManager` now holds `_relayer`, set by the host admin through
`setRelayer`, and `onAccept` reverts with `UnauthorizedRelayer` for any other relayer, zero
included. Only governance traffic reaches this contract, so ordinary relaying is unaffected.
No file in this package changed; the entry is here because the delivery flow documented in
`Flow.md` is what it corrects.

Files: `evm/src/core/HostManager.sol`, `evm/script/DeployIsmp.s.sol`,
`evm/script/DeployHostManager.s.sol`, `evm/tests/foundry/HostManagerTest.sol`,
`evm/tests/foundry/BaseTest.sol`, `evm/tests/rust/src/tests/utils.rs`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`.

## 2026-09-03 — Relayer allowlist on `HyperFungibleToken`, fail-closed on the BRIDGE token

`HyperFungibleToken` gains `_relayer`, `relayer()`, `setRelayer(address)` (owner only), the
`RelayerUpdated(address previous, address current)` event, the `UnauthorizedRelayer` error, and a
virtual `_checkRelayer` that `onAccept` and `onPostRequestTimeout` call before anything else. Both
callbacks mint, so both are gated. In the base the check is opt-in: zero leaves deliveries open, so
tokens already deployed from this package behave as before until their owner sets a relayer.
`evm/src/apps/BridgeToken.sol` overrides `_checkRelayer` to fail closed, and its deploy script sets
the relayer from `GOVERNANCE_RELAYER` before `configure`, so the token is never live without one.
`IHyperFungibleToken` is unchanged: `supportsInterface` keys on its `interfaceId`, so adding the
new functions there would change what every existing deployment reports.

Files: `contracts/apps/HyperFungibleToken.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/BridgeToken.sol`,
`evm/script/DeployBridgeToken.s.sol`, `evm/tests/foundry/BridgeTokenTest.t.sol`,
`evm/tests/foundry/HyperFungibleTokenTest.sol`.

## 2026-09-03 — Relayer allowlist on the intent gateway

The gateway now accepts `onAccept` and `onGetResponse` deliveries only from a single authorised
relayer stored at `_relayer` (slot 13, packed behind `_paused`). The check runs before the message
body is decoded, so escrow redemptions, refunds and every governance action, upgrades included, are
covered. A refused delivery reverts, which the host records as undelivered, so the authorised
relayer can submit the same message later. `setRelayer(address)` is callable by the immutable
`_owner` and by the host; the host branch exists so a governance `UpgradeContract` can carry the
call as its migration calldata and arm the relayer in the upgrade transaction (`upgradeToAndCall`
delegatecalls that calldata with the host still as `msg.sender`).

The interface gains `RelayerUpdated(address previous, address current)` and `setRelayer`, keeping
its declarations identical to `IntentsBase`. The unused `_paused` getter was dropped from the gateway
to stay under the EIP-170 size limit; it was never declared here.

Files: `contracts/apps/IntentGatewayV2.sol`, `package.json`, `docs/ai/ChangeLog.md`,
`docs/ai/Decisions.md`, `docs/ai/Flow.md`. Gateway side: `evm/src/apps/IntentGatewayV2.sol`,
`evm/src/apps/intentsv2/IntentsBase.sol`, `evm/src/apps/intentsv2/ExtrinsicIntents.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`.

## 2026-08-27 — `IIntentGatewayV2` brought back in sync with the gateway (#1160)

`IIntentGatewayV2` had drifted from the deployed `IntentGatewayV2`. Every declaration in the
interface is now identical to the one in `evm/src/apps/intentsv2/IntentsBase.sol`, verified by
diffing the two declaration sets:

- `OrderFilled`, `EscrowReleased` and `EscrowRefunded` were still the pre-`tokens` signatures. All
  three take a `TokenInfo[]` the interface did not declare.
- `NewDeploymentAdded(bytes stateMachineId, address gateway)` does not exist. The gateway emits
  `DeploymentAdded(string chain, address gateway)` — wrong name and wrong parameter type, so a
  consumer filtering on it would have matched nothing.
- `PartialFill`, `DestinationProtocolFeeUpdated`, and the `UnknownInstance` and
  `PartialFillNotAllowed` errors were absent.
- `OrderCancelled(bytes32 indexed commitment, address canceller)` was added, the event this issue
  introduces on the gateway.

Nothing else in the repo compiles against these declarations — `SolverAccount.sol` imports the
interface only for `select.selector` and `fillOrder.selector` — so the change is inert here and
matters to integrators who read the interface as the gateway's published surface.

Files: `contracts/apps/IntentGatewayV2.sol`, `package.json`, `docs/ai/ChangeLog.md`,
`docs/ai/Decisions.md`, `docs/ai/Flow.md`.
