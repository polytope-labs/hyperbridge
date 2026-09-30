# 2026-09-03 — Zero relayer fails closed, and the upgrade arms it atomically

Superseded on 2026-09-05: an unset relayer gates nothing, see above.

Chosen: an unset `_relayer` matches no delivery, because the handler always forwards a real
`msg.sender`. The rollout sets it in the upgrade transaction via `upgradeToAndCall` calldata.

Alternative rejected — treat zero as "allowlist disabled". Convenient for tests and a forgotten
init, but it makes the safe state opt-in, and a fresh proxy would run unguarded until someone
noticed. A refused delivery costs nothing: the host deletes the receipt and the authorised relayer
can resubmit.

Alternative rejected — a `reinitializer(2)` taking the relayer. It is one-shot, so rotation would
need the setter anyway, and it does not solve who may call it.
