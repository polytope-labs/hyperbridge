# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/sdk`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-18 — `SigningAccount` describes only what the SDK calls

Chosen: drop `signMessage` from `SigningAccount`, leaving `signRawHash` and `signTypedData`.

Alternative considered: leaving it in place, since removing a member from a published interface is a public API change.

Why: the interface exists so a caller can hand the SDK a signing backend, and every member it declares is a cost paid by every implementer. `signMessage` was never invoked — bid UserOperations are EIP-712-signed through `signTypedData` — so the cost bought nothing, and it forced downstream signer abstractions (notably `@hyperbridge/simplex`'s `Signer`) to carry a dead method into their own public surface. The narrowing is safe in the direction that matters: implementers with an extra method still satisfy the type, and only a caller of `solverSigner.signMessage` would break, of which there are none.

`signRawHash` stays despite also being uncalled here: it is genuinely part of what a solver signer must provide (simplex signs the EIP-7702 authorization digest with it), so the interface still describes something real.
