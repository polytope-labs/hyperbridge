# ChangeLog

AI-maintained log of code changes in `sdk/packages/core`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/core/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-09-03 — Relayer allowlist on `HyperFungibleToken`, fail-closed on the BRIDGE token

`HyperFungibleToken` gains `_relayer`, `relayer()`, `setRelayer(address)` (owner only), the
`RelayerUpdated(address previous, address current)` event, the `UnauthorizedRelayer` error, and a
virtual `_checkRelayer` that `onAccept` and `onPostRequestTimeout` call before anything else. Both
callbacks mint, so both are gated. In the base the check is opt-in: zero leaves deliveries open, so
tokens already deployed from this package behave as before until their owner sets a relayer.
`evm/src/apps/BridgeToken.sol` overrides `_checkRelayer` to fail closed, and its deploy script sets
the relayer from `BRIDGE_RELAYER` before `configure`, so the token is never live without one.
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
