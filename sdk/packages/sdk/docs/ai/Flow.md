# Flow

AI-maintained map of how code paths in `sdk/packages/sdk` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## How a solver's bid gets signed

`SubmitBidOptions.solverSigner` is a `SigningAccount` (`src/types/index.ts`), supplied by the caller. `@hyperbridge/simplex` does not pass its `Signer` directly: the payload parameter types differ (`unknown` here, `TypedDataPayload` there), so its `ContractInteractionService` adapts at the call site with `sdkSigningAccount(signer)`.

1. The caller assembles the bid and calls `prepareSubmitBid` (`src/protocols/intents/BidManager.ts`), passing the solver account, nonce, entry point, gas limits, pre-built ERC-7821 `callData`, and any `paymasterAndData`.
2. `BidManager` builds the v0.7-packed `PackedUserOperation` with an empty signature, then checks the nonce key binds the order commitment and session key (`CryptoUtils.bidNonceKey`) — a mismatch is warned about, not thrown, and fails on-chain validation later.
3. It signs `CryptoUtils.packedUserOpTypedData(userOp, entryPointAddress, chainId)` with **`solverSigner.signTypedData`** — the only `SigningAccount` member this package calls, and it passes the payload alone: the chain id the backend might need is already in `domain.chainId`. Signing the typed data rather than the digest produces the same signature `SolverAccount._rawSignatureValidation` recovers, while leaving the payload legible to a custody backend's policy engine.
4. The returned signature is prefixed with the order id (`concat([order.id, solverSignature])`) — that concatenation, not the bare signature, is what goes on the UserOperation.

`signTypedData` is the interface's only member: `signMessage`, `signRawHash`, and `signTypedData`'s chain-id argument were all removed on 2026-08-18 as uncalled. `GasEstimator`'s `signMessage` call is viem's method on a locally derived account, unrelated to `SigningAccount`.
