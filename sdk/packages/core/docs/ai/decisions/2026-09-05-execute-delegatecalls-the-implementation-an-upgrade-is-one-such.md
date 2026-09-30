# 2026-09-05 — `Execute` delegatecalls the implementation; an upgrade is one such call

Chosen: one governance action that delegatecalls the proxy's current implementation with the
body as calldata. `msg.sender` stays the host, so the existing `onlyHost` guards are the whole
access model, and `upgradeToAndCall` becomes an ordinary host-only function. A rotation no longer
has to be dressed up as an upgrade to the same implementation.

Alternative rejected — an external self-call, `address(this).call(data)`. It makes the gateway
its own caller, so every governance target would need an `onlySelf` guard in place of
`onlyHost`, and the gateway as caller holds the escrow, which the host as caller does not.

Alternative rejected — a `SetRelayer` action next to `UpgradeContract`. One more variant to
mirror in the pallet for one function; `Execute` covers it and anything host-only added later.

Kept — `UpgradeContract` in the pallet, under the same discriminator. The live implementation
understands only that body, so it is the one message that can install this code; afterwards it
selects no function here and reverts, which `testLegacyUpgradeBodyIsRefused` pins so the mistake
is loud rather than silent.

Cost accepted — `Execute` can call any function of the implementation with the host as sender,
not only the host-only ones. Governance can already install arbitrary code through an upgrade,
so this widens nothing.
