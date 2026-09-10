# 2026-08-28 — Decode phantom bids of either `fillOrder` shape

`extractFillDataVm2` built one ethers `Interface` from `FILL_ORDER_ABI`. When `FillOptions` gained `validUntil`
that ABI moved to the v2 shape, and since ethers validates the selector before decoding, every v1-shaped bid
started throwing. The throw was swallowed by a `continue`, the function returned null, and `aggregatePhantomBids`
drops a null bid with no log line — so the loss was silent.

That is not a future risk: `@hyperbridge/sdk` is a `workspace:*` dependency, so the indexer picked up the v2 ABI
immediately, while every mainnet gateway still runs the pre-`validUntil` implementation. `getFillOptionsVersion`
therefore returns 1 and simplex encodes v1 bids, all of which were being discarded — no bidder rows, no median,
no pool-rate snapshot from any of them.

Now tries both shapes, mirroring the SDK's `decodeFillOrder`. The selectors differ (`0x5cfb1ea5` vs
`0xa5470064`), so neither can mis-decode the other's payload. The v1 ABI is imported from the SDK rather than
re-declared here, so the two cannot drift apart again.

Verified by running the real function against both shapes: before the change v1 returned null and v2 decoded;
after, both decode, and a wrong target or non-`fillOrder` calldata still returns null.

Also adds `transformIgnorePatterns` to the jest config. The SDK's CJS bundles `require()` ESM-only packages
(`p-queue` -> `eventemitter3`/`p-timeout`, `lodash-es`), which jest cannot parse untransformed, so *any* test
importing `@hyperbridge/sdk` or its `intents-helpers` sub-path failed to load at all against a freshly built SDK.
That was blocking the new test, and it was also silently keeping the existing `phantom-decode.test.ts` from
running — that file passes again now.

Files: `src/utils/phantom-decode.ts`, `src/utils/__tests__/phantom-decode.fill.test.ts`, `jest.config.ts`.
