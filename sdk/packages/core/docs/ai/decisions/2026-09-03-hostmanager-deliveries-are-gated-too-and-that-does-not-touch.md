# 2026-09-03 — `HostManager` deliveries are gated too, and that does not touch permissionless relaying

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
