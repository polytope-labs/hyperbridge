# 2026-09-05 — The relayer gate belongs to the token that needs it, not to `HyperFungibleToken`

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
