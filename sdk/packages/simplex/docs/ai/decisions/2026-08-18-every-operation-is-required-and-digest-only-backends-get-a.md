# 2026-08-18 — Every operation is required, and digest-only backends get a factory instead of optionality

Chosen: `signTypedData`, `signAuthorization` and `signTransaction` are all required, `mode` with them. `signRawHash` is deleted. `digestSigner({ address, mode, sign })` builds a `Signer` from a single `sign(hash)`.

Alternatives considered: optional `signAuthorization`/`signTransaction` with `signRawHash` as the always-present fallback (the previous cut); requiring them with no factory.

Why: with the structural methods optional, the interface said two contradictory things — "tell me what you can do" and "here is a digest, never mind". Requiring them makes the contract one thing: these are the three operations a solver needs authorised, encode each however your backend can. The cost is that a digest-only backend now has to hash an EIP-7702 authorization and serialise an EIP-1559 transaction, which is real work and exactly where a subtle bug produces a valid signature over the wrong bytes — so that work is not pushed outward, it is packaged as `digestSigner`. The one-liner an HSM integration needs is unchanged; it just goes through a factory instead of leaving holes in the interface.

Note what required-ness deleted: with both structural methods guaranteed, `signRawHash` had no caller left — not in `DelegationService`, not in `accountFor`, not in the sdk (which never called it). Keeping it "optional" would have re-created the `signMessage` situation: a member on the published interface that nothing invokes. It is removed from `SigningAccount` in the sdk too, leaving that interface with the one method the sdk actually calls.
