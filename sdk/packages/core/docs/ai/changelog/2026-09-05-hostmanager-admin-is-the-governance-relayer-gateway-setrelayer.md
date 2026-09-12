# 2026-09-05 — `HostManager` admin is the governance relayer; gateway `setRelayer` is host-only

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
