# 2026-08-18 — `signMessage` and the `chainId` argument dropped, in both packages

Chosen: neither `Signer` nor the sdk's `SigningAccount` declares `signMessage`, and `signTypedData` takes the payload alone.

Alternatives considered: keeping both, since `Signer` extended `SigningAccount` and the sdk declared them.

Why: nothing called `signMessage`. Bids are signed as EIP-712 UserOperations through `signTypedData` (`BidManager.prepareSubmitBid`), and the sdk's own interface declared it without ever invoking it, so the requirement propagated out to every implementer for nothing. The `chainId` argument was the same shape of problem: EIP-712 puts the chain id in `domain.chainId`, which is what the digest covers, and every viem-backed adapter took the argument and discarded it. Its one consumer was MPCVault's request envelope — which its own account wrapper already derived from the payload — and the adapter defaulted a missing value to `1`, so a call site that forgot would have had the vault authorise a signature under mainnet. Reading the domain and throwing when it is absent replaced that.

Removing both from the sdk is safe in the direction that matters: type narrowing breaks callers of the removed member, and there are none. `Signer` no longer extends `SigningAccount` (the payload types differ in variance); `sdkSigningAccount(signer)` adapts at the two call sites that hand a signer to the sdk.
