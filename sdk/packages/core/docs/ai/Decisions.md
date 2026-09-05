# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/core`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-09-05 — The relayer gate belongs to the token that needs it, not to `HyperFungibleToken`

Chosen: `HyperFungibleToken` has no relayer state, setter, event, error or hook. Its two delivery
callbacks are `public virtual`, and `BridgeToken` wraps them with its own gate. A token that wants
a gate writes one; a token that does not gets nothing to configure and no extra storage slot.

The opt-in gate in the base contract (zero means open, override to fail closed) was two policies
in one place: third-party tokens saw a setter they had no reason to call, and the one token that
needed the gate had to override the hook to invert its default. Moving the whole thing into
`BridgeToken` leaves one policy per contract.

Alternative rejected — keep an empty `_checkRelayer` hook in the base for derived tokens to fill
in. It still names a relayer in a contract that has no opinion about one, and wrapping the
callbacks costs the derived token nothing more than a `super` call.

## 2026-09-05 — `version()` is the implementation's `VERSION`; `initialize` arms, `migrate` catches up

Chosen: one constant, `VERSION = 2`, that both `initialize` and `migrate` land on. `initialize`
takes the relayer in the init data, so a fresh proxy is armed from its first block and reports
the version of the code it runs. `migrate`, host-only and one-shot, exists for proxies deployed
before this implementation and reverts on any proxy already at `VERSION`. `setRelayer` is a
rotation and never touches the version. `onlyHost` on `migrate` is load-bearing: a proxy at 1 is
open, and without it anyone could arm it first.

This reverses the earlier decision (below, same day) to keep the relayer out of the init data.
The reason given there, that the relayer would become part of what fixes the proxy's CREATE2
address, still holds but no longer bites: the implementation address is already an input to
that address, so every new implementation changes it for chains deployed afterwards anyway, and
the deploy script now deploys a proxy only where none exists. Landing fresh proxies at 1 with an
open gate, the state before this change, left them reporting an older version than their code.

Alternative rejected — `setRelayer` under `reinitializer(_getInitializedVersion() + 1)`, built
and tested first. It makes `version()` count key rotations, which says nothing about what code a
proxy has migrated to.

Alternative rejected — a fixed `reinitializer(2)` on `setRelayer` itself: the second rotation
reverts until an implementation with `reinitializer(3)` ships.

Alternative rejected — a separate `bumpVersion()` next to a plain `setRelayer`: an
`UpgradeContract` carries one migration call, so arming a fresh chain would take two deliveries.

The bytes came from deduplicating internal code, not from dropping anything off-chain reads. The
one place `_sendValue` is not used is the `_fillSameChain` loop, which is at the via-ir stack
limit.

## 2026-09-05 — The `HostManager` admin is the relayer, rotated only by governance

Chosen: `HostManagerParams.admin` survives `init` and is the address `onAccept` compares the
relayer against. There is no separate relayer slot and no local setter; a `SetAdmin` request from
Hyperbridge, delivered by the outgoing admin, replaces it.

The previous design gave the host admin a `setRelayer` on the manager. That key could re-route
governance at will, from a local transaction nobody on Hyperbridge sees. Folding the relayer into
the admin removes that path: the only way to change who may deliver governance is governance, and
the manager's admin has exactly one power after `init`, which is to deliver.

Alternative rejected — keep `setRelayer` but restrict it to the manager's own admin. Same local
override, different key.

Alternative rejected — a zero admin as a kill switch, as a zero relayer was. A zero admin can never
be rotated away: the rotation is itself a delivery the manager would refuse, and the host cannot be
re-pointed at a new manager except through the current one. Zero is refused in the constructor and
in `SetAdmin`, and the pallet refuses to dispatch it; the cost is a comparison each, the failure
they prevent is permanent.

## 2026-09-05 — `init` is one-shot, and the EVM deploy scripts do not call it

Chosen: `init` binds the host only while it is unset; a manager constructed with its host set is
bound already.

With the admin surviving `init`, a repeatable `init` would let the governance relayer key re-point
the host and cut it off from its own governance. One-shot closes that. `DeployIsmp.s.sol`
therefore constructs the host first, which works because `EvmHost` takes its params in
`initialize` rather than its constructor, and passes the host to the manager's constructor, so the
relayer key never has to sign a deploy transaction. `TronHost` takes its params in the constructor,
so the Tron migration keeps the `init` route with the deployer as admin until governance rotates
it.

Alternative rejected — precompute the host's CREATE2 address in the script. Works, but couples the
script to the CREATE2 deployer and the host's creation code for no gain over reordering.

## 2026-09-05 — Gateway `setRelayer` is host-only and the only writer; unset means open

Chosen: `setRelayer` lives in `ExtrinsicIntents` behind `onlyHost`, `initialize` does not touch the
relayer, and `_checkRelayer` passes every delivery while `_relayer` is zero. This supersedes the
2026-09-03 decision below that zero fails closed.

The owner branch existed so a fresh proxy could be armed locally; it also let the owner key
redirect every cross-chain delivery without a governance message. Removing it leaves the host as
the only caller, reachable solely from `UpgradeContract` migration calldata. Carrying the relayer
in the init data was tried and rejected: the relayer is operational state that governance owns,
not part of what fixes a proxy's address. With no local or init-time arming left, the message that
arms a fresh proxy is a governance delivery, so the unarmed proxy has to accept it; an unset
relayer therefore gates nothing, and `setRelayer(address(0))` reopens the gate. The window is the
one between deployment and the `upgrade_gateway` that arms it, and closing it is governance's
first act on a new chain.

Alternative rejected — a `RequestKind.SetRelayer` governance action instead of the host-only
function. Simpler to invoke, but it needs the `intents-coprocessor` pallet mirrored, and the
migration-calldata route already exists and is tested.

`_owner` stays as the placeholder it was before the relayer work, with nothing to do.

## 2026-09-05 — `relayer()` on both contracts, `version()` on the gateway from `Initializable`

Chosen: the manager and the gateway both answer `relayer()`. The gateway's `version()` returns
`_getInitializedVersion()` from OpenZeppelin's `Initializable`: 1 once `initialize` has run, and
raised only by a `reinitializer(n)` migration, so it tracks the storage-level migrations a proxy
has been through rather than a number someone has to remember to bump. Implementations from
before the gate have no `version()` at all and revert. The manager is not upgradeable and gets no
`version()`; a manager without `relayer()` is one from before the admin became the relayer.

The underscore-public convention in `IntentsBase` (`_filled`, `_orders`, ...) would have given
`_relayer()` for free, but a named getter is what the interface should publish, and the gateway
had 26 bytes of headroom: `relayer()` only fits once the auto-generated getter is dropped, and
`version()` needed the `_instances` getter dropped as well. That getter duplicated
`instance(bytes)` and nothing off-chain called it.

Alternative rejected — a hand-maintained version constant, as a semver string or a number. The
string cost 82 bytes and did not fit; both would drift from what is actually deployed.

## 2026-09-03 — `HostManager` deliveries are gated too, and that does not touch permissionless relaying

Chosen: `HostManager.onAccept` refuses any relayer but the one the host admin set, zero included.

The app gates check an address the host reports, and the host takes it from its handler. The
handler is a host parameter that a `SetHostParam` governance message can replace, and until now
any relayer could deliver that message once its consensus proof verified. Under a forged consensus
an attacker would swap in a handler that reports the whitelisted relayer on every message, and the
app gates would pass. Gating the HostManager closes that route.

It does not weaken the open-relayer model because the HostManager never carries user traffic. Its
first check already rejects anything not sourced from Hyperbridge, so the only messages it ever
sees are Polytope's own `Withdraw` and `SetHostParam`. Third-party relayers keep delivering every
ordinary message to every ordinary app exactly as before.

Alternative rejected — a delay on host parameter changes with the admin freeze as a veto. Keeps
delivery open but needs someone watching every chain, and adds latency to legitimate governance.

Alternative rejected — move handler and consensus-client changes to the host admin key. Simplest,
but it takes that power away from cross-chain governance rather than protecting it.

The setter authority is the host admin rather than the HostManager's own admin, which is zeroed
after `setIsmpHost`. The host admin is the key that can already freeze the host and reset its
consensus state, so no new trust is introduced. Zero is checked explicitly here since there is no
bytecode pressure; on the gateway the same guarantee comes from the handler never forwarding zero.

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

Superseded on 2026-09-05: an unset relayer gates nothing, see above.

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
