# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/core`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-09-03 — `HyperFungibleToken` opts in to the relayer gate; `BridgeToken` fails closed

Chosen: the base token's `_checkRelayer` only rejects when a relayer has been set, and
`BridgeToken` overrides it so that an unset relayer matches nobody.

`HyperFungibleToken` is a library contract that third parties deploy from this package. Failing
closed there would leave every token deployed without a `setRelayer` call unable to receive
anything, with no compile-time signal. The BRIDGE token is ours, its supply is backed by the nexus
escrow, and a forged mint is exactly the attack the gate exists for, so it takes the strict
semantics of the intent gateway. The two behaviours live in one virtual function so the difference
is visible in one place rather than spread through the callbacks.

Alternative rejected — override `onAccept` in `BridgeToken`. It is `external`, so an override
cannot call the parent body and would have to duplicate the mint logic.

Alternative rejected — make the base fail closed and bump the package major. Correct in principle,
but the request was for the bridge token, and the base can be tightened later once every
deployment from this package has a relayer set.

## 2026-09-03 — `onPostRequestTimeout` is gated on the token, unlike the gateway

Timeouts mint a refund to the original sender, so a forged timeout proof mints. The intent gateway
does not gate its timeout callbacks because `HyperApp`'s defaults revert and it dispatches with no
timeout.

## 2026-09-03 — `IHyperFungibleToken` does not declare `setRelayer` or `relayer`

`supportsInterface` returns true for `type(IHyperFungibleToken).interfaceId`. Extending the
interface changes that id, so existing deployments would stop matching it and new ones would report
an id integrators have not seen. The relayer functions are reachable through the contract type.

## 2026-09-03 — The relayer is a separate storage variable, not a `Params` field

Chosen: `address _relayer` appended after `_paused` in `IntentsBase`, set through its own
`setRelayer` call.

`Params` occupies slots 4 to 8 and `_orders` starts at slot 9. Adding a field to the struct would
push every mapping behind it and corrupt escrow on the live proxy. Reusing `UpdateParams` was
therefore never available, quite apart from it being a Hyperbridge-relayed message: the whole point
is to hold even if Hyperbridge's consensus is compromised, so the setter must not depend on it.

Alternative rejected — a new `RequestKind` for governance to set the relayer. It needs the
`intents-coprocessor` pallet mirrored, and it is still a cross-chain message. Governance can already
rotate the relayer through `UpgradeContract` migration calldata when it wants to; the owner path
covers the case where it cannot.

## 2026-09-03 — Zero relayer fails closed, and the upgrade arms it atomically

Chosen: an unset `_relayer` matches no delivery, because the handler always forwards a real
`msg.sender`. The rollout sets it in the upgrade transaction via `upgradeToAndCall` calldata.

Alternative rejected — treat zero as "allowlist disabled". Convenient for tests and a forgotten
init, but it makes the safe state opt-in, and a fresh proxy would run unguarded until someone
noticed. A refused delivery costs nothing: the host deletes the receipt and the authorised relayer
can resubmit.

Alternative rejected — a `reinitializer(2)` taking the relayer. It is one-shot, so rotation would
need the setter anyway, and it does not solve who may call it.

## 2026-09-03 — `setRelayer` accepts the host, not `address(this)`

Chosen: `msg.sender == _owner || msg.sender == host()`.

The first draft allowed `address(this)`, expecting `upgradeToAndCall` to call the proxy. It does
not: it delegatecalls the migration calldata, so `msg.sender` is still the host that invoked
`onAccept`. The fork test against the live mainnet proxy failed with `Unauthorized` and exposed it.
Accepting the host adds no trust: the host already gates every callback, and it never calls the
gateway with any selector other than the `IApp` callbacks, so the branch is reachable only from
governance migration calldata that has already passed the hyperbridge-source check and the relayer
gate.

## 2026-09-03 — The `_paused` getter was dropped to stay under EIP-170

Chosen: `bool internal _paused`. The variable stays in slot 13 so the layout is unchanged.

The gateway compiled to 10 bytes over the limit with the new storage, setter, event and checks.
Nothing reads `_paused()` anywhere in the repo, so removing its getter was the only free saving.
The revert reuses `Unauthorized()` instead of a dedicated error for the same reason; the second
selector cost 15 bytes.

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
