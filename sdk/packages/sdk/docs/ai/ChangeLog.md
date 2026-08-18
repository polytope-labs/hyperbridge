# ChangeLog

AI-maintained log of code changes in `sdk/packages/sdk`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/sdk/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-18 — `SigningAccount` drops `signMessage` and the `chainId` argument

`SigningAccount` is the contract a solver's signing backend satisfies to submit bids (`SubmitBidOptions.solverSigner`). It declared `signMessage(messageHash, chainId)`, which nothing in this package ever called: bids are signed as EIP-712 UserOperations in `BidManager.prepareSubmitBid` via `signTypedData`, and `GasEstimator`'s one `signMessage` call is viem's own method on a locally derived account, not this interface. The requirement propagated out to every implementer — including `@hyperbridge/simplex`, whose public `Signer` extends this type — so removing it here is what let that interface shrink to what a solver actually needs.

`signTypedData`'s second parameter went the same way. EIP-712 carries the chain id in `domain.chainId` — that is what the digest covers — and `BidManager` was passing it alongside a payload that already contained it, for the benefit of no implementation in this package.

Type-only narrowing: it can break a caller of a removed member, and there are none inside the workspace.

Files: `src/types/index.ts`, `src/protocols/intents/BidManager.ts`.
