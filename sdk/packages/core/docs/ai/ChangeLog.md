# ChangeLog

AI-maintained log of code changes in `sdk/packages/core`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/core/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

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
