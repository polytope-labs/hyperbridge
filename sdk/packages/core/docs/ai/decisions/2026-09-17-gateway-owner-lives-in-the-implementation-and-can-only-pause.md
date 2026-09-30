# 2026-09-17 — Gateway owner lives in the implementation and can only pause

Chosen: the implementation inherits OpenZeppelin's `Ownable2StepUpgradeable`, and the owner's only
power is pausing the gateway. Relayer rotation, params, dust and upgrades stay with governance, as
the 2026-09-05 decision on `setRelayer` settled: an owner key that could redirect deliveries without
a governance message is what that decision removed. Pausing moves no funds.

The pause covers `placeOrder`, `fillOrder` and the escrow-moving host callbacks. It exempts
deliveries whose source is Hyperbridge itself, so governance can still upgrade, update params or
replace the owner while paused. `_checkOwner` accepts the host too, so a lost or renounced owner key
cannot leave the gateway paused: governance can unpause or install a new owner. `cancelOrder` stays
open so users can always start a refund. Refused deliveries are not lost: the host deletes the
receipt of a reverted callback, and gateway requests have no timeout.

`Ownable2StepUpgradeable` keeps its storage at ERC-7201 namespaced slots, so it adds nothing to the
sequential layout `IntentsBase` shares with the modules. In 5.6 it imports `Initializable` from
`@openzeppelin/contracts`, the same contract the gateway already inherits.

The pause is `PausableUpgradeable`, for its `whenNotPaused` modifier, events and errors, and its flag
is namespaced too. `onAccept` calls `paused()` inline instead of taking the modifier, because
governance deliveries must still land.

The old `bool _paused` in `IntentsBase` is removed rather than left behind as dead storage. It sat at
slot 13 offset 0 with `_relayer` packed at offset 1, so `_relayer` moves to offset 0 and `migrate`
moves it on existing proxies: `sstore(13, sload(13) >> 8)`. The shift needs no input, so the upgrade
calldata cannot carry a wrong relayer, and the old byte falls off the end. The upgrade is already
atomic, `upgradeToAndCall(impl, migrate(owner))`, and the relayer gate for that delivery runs on the
old implementation, so nothing reads `_relayer` between the move and the new code. The cost is that
the 2 to 3 upgrade must carry `migrate`; `intentGatewayUpgradeInitialization` always builds it for a
proxy at 2.

Alternative rejected — keeping `IntentsBase._paused` as the flag. It works, but hand-writes the
modifier, events and errors `PausableUpgradeable` already provides.

Alternative rejected — keeping the byte as an unread placeholder. Safe with no migration step, but it
leaves a dead field in the layout the modules share, for a one-time shift in a `migrate` that the
upgrade runs anyway.

Alternative rejected — `migrate(owner, relayer)`, re-setting the relayer from calldata. Governance
would have to supply each chain's relayer correctly, and a wrong one refuses every later delivery,
governance included.

Alternative rejected — the non-upgradeable `Ownable2Step` with its functions overridden onto a
namespaced slot. Its `_owner` and `_pendingOwner` stay declared as sequential storage, landing at
slots 15 and 16 in the implementation only, which fails the module layout test's variable count,
and its constructor writes an owner into the implementation contract's own storage.

Alternative rejected — a hand-written namespaced owner. It worked, but duplicates OpenZeppelin's
contract, events and errors.

Alternative rejected — a sequential owner in `IntentsBase` or `IntentGatewayV2`. In `IntentsBase`
it joins the layout the modules share for a value no module reads; in `IntentGatewayV2` the next
field appended to `IntentsBase` would take its slot and silently move it.
