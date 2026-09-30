# 2026-09-08 — EIP-2612 is the bootstrap authorization, and nothing else

Chosen: the `permitBootstrap` flag, set only by `DelegationService`'s first-time delegation
op, lets that one op authorize with an EIP-2612 permit when the fee token has no Permit2
allowance yet. The same op carries `approve(Permit2, max)` in its callData. Every other
sponsored op — fills, bids, vault sweeps, token sends — authorizes through Permit2 with no
way to reach the permit path.

Alternatives considered: dropping 2612 outright (the previous entry) and accepting that a
fresh solver needs native dust per chain; or restoring 2612 as the general preference for
permit-capable tokens, as it was before.

Why the scoping is the whole design. The objection to 2612 is its nonce: one sequential
counter per owner, so two permits signed for the same solver carry the same value and only
one lands. That is fatal for fills, which are the ops that actually run concurrently. It is
free for the delegation, which happens once per chain and provably has no concurrent
sibling — the account is not even delegated yet. Confining the permit to that op keeps the
serialization hazard away from everything that could hit it.

Dropping it outright lost more than it looked. The permit is the only authorization that
needs no prior on-chain state, so it is the only way an account with stablecoins and zero
native can pay for anything. Requiring native dust per chain sounds minor until it is the
operator's first run on a new chain and the failure is "send ETH here" rather than "it
worked".

Putting the approve in the same op's callData is what makes it worth doing. A permit-only
delegation would leave the account delegated but still without a Permit2 allowance, so the
next op would need the native approve anyway — the permit would have bought one op's delay,
not a bootstrap. Because the paymaster prefunds during validation and callData runs after,
the permit covers gas without the allowance existing, and the allowance exists by the time
anything else needs it.

Making it a fallback rather than a preference matters too: with the flag set and an
allowance already in place, the builder still picks Permit2. The permit is for the state
where Permit2 cannot work, not for tokens that happen to support it.
