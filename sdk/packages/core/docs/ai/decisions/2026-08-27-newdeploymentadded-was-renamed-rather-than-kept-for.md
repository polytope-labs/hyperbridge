# 2026-08-27 — `NewDeploymentAdded` was renamed rather than kept for compatibility

Chosen: the interface's `NewDeploymentAdded(bytes stateMachineId, address gateway)` was replaced
with the gateway's actual `DeploymentAdded(string chain, address gateway)`.

Renaming a declaration in a published interface normally breaks consumers. It does not here,
because there is nothing to break: no deployed gateway has ever emitted `NewDeploymentAdded`, so
anyone filtering on that topic has been matching zero logs. Keeping it would preserve a name that
only ever produces silence, next to the real one.

Alternative rejected — declare both. The interface would then advertise an event the contract
cannot emit, which is the state that caused this in the first place.
