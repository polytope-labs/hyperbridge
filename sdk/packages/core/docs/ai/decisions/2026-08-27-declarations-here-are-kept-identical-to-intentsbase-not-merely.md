# 2026-08-27 — Declarations here are kept identical to `IntentsBase`, not merely compatible

Chosen: every event and error in `IIntentGatewayV2` matches `evm/src/apps/intentsv2/IntentsBase.sol`
exactly — same name, same parameter types, same `indexed` flags.

Nothing compiles against these declarations (`SolverAccount.sol` uses the interface only for two
function selectors), so a mismatch produces no build error anywhere in the repo. That is exactly
why the drift went unnoticed through several signature changes. The only thing that can catch it
is the rule that the two lists are equal, which is cheap to check by diffing them.

Alternative rejected — declare only the subset integrators are expected to use. It sounds tidier,
but it makes "missing from the interface" ambiguous: you cannot tell a deliberate omission from
another four events nobody updated.
