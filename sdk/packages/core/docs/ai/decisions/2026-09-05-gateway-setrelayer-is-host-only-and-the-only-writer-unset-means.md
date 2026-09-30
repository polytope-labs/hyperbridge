# 2026-09-05 — Gateway `setRelayer` is host-only and the only writer; unset means open

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
