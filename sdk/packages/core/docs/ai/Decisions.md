# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/core`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-27 — `NewDeploymentAdded` was renamed rather than kept for compatibility

Chosen: the interface's `NewDeploymentAdded(bytes stateMachineId, address gateway)` was replaced
with the gateway's actual `DeploymentAdded(string chain, address gateway)`.

Renaming a declaration in a published interface normally breaks consumers. It does not here,
because there is nothing to break: no deployed gateway has ever emitted `NewDeploymentAdded`, so
anyone filtering on that topic has been matching zero logs. Keeping it would preserve a name that
only ever produces silence, next to the real one.

Alternative rejected — declare both. The interface would then advertise an event the contract
cannot emit, which is the state that caused this in the first place.

## 2026-08-27 — Declarations here are kept identical to `IntentsBase`, not merely compatible

Chosen: every event and error in `IIntentGatewayV2` matches `evm/src/apps/intentsv2/IntentsBase.sol`
exactly — same name, same parameter types, same `indexed` flags.

Nothing compiles against these declarations (`SolverAccount.sol` uses the interface only for two
function selectors), so a mismatch produces no build error anywhere in the repo. That is exactly
why the drift went unnoticed through several signature changes. The only thing that can catch it
is the rule that the two lists are equal, which is cheap to check by diffing them.

Alternative rejected — declare only the subset integrators are expected to use. It sounds tidier,
but it makes "missing from the interface" ambiguous: you cannot tell a deliberate omission from
another four events nobody updated.
