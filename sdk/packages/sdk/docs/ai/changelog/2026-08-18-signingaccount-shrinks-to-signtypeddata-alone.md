# 2026-08-18 — `SigningAccount` shrinks to `signTypedData` alone

`SigningAccount` is the contract a solver's signing backend satisfies to submit bids (`SubmitBidOptions.solverSigner`). It declared `signMessage(messageHash, chainId)`, which nothing in this package ever called: bids are signed as EIP-712 UserOperations in `BidManager.prepareSubmitBid` via `signTypedData`, and `GasEstimator`'s one `signMessage` call is viem's own method on a locally derived account, not this interface. The requirement propagated out to every implementer — including `@hyperbridge/simplex`, whose public `Signer` satisfied this type (it no longer extends it; simplex adapts with `sdkSigningAccount` at the two call sites) — so removing it here is what let that interface shrink to what a solver actually needs.

`signTypedData`'s second parameter went the same way. EIP-712 carries the chain id in `domain.chainId` — that is what the digest covers — and `BidManager` was passing it alongside a payload that already contained it, for the benefit of no implementation in this package.

`signRawHash` followed in the same day's follow-up: once simplex's `Signer` required `signAuthorization` and `signTransaction`, no caller in either package handed a raw digest to the interface, leaving `signTypedData` as the whole contract.

Type-only narrowing: it can break a caller of a removed member, and there are none inside the workspace.

Files: `src/types/index.ts`, `src/protocols/intents/BidManager.ts`.
