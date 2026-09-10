# 2026-09-05 — The `HostManager` admin is the relayer, rotated only by governance

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
