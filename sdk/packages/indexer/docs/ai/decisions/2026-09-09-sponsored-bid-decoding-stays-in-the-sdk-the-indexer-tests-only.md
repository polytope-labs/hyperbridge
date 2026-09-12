# 2026-09-09 — Sponsored-bid decoding stays in the SDK; the indexer tests only its own seams

Chosen: the second paymasterAndData shape (Permit2 sponsorship, declaration appended) is handled inside
`aggregatePhantomBids` in the SDK, and the indexer changes no handler or util. Its tests cover the two places the
indexer substitutes its own implementation for the SDK's: the ethers userOpHash recovery, which now hashes a
234-byte-plus `paymasterAndData`, and the `intents-helpers` bundle it ships, checked against a payload packed with
ethers rather than viem.

Alternative rejected — a VM2-safe copy of the decoder in `phantom-decode.ts`, next to the fill and signature
helpers. Those exist because viem's byte handling throws in the SubQuery sandbox; the declaration decoder uses
`@polkadot/util` only, which already runs there today for the bare shape, so a copy would duplicate a security-
relevant parser for no sandbox reason and drift from the SDK's.

Alternative rejected — persisting the sponsorship fields (paymaster, fee token, permit nonce) on the bid or
snapshot rows. Nothing reads them: a phantom bid never executes, so which paymaster it named and when its permit
expires are not facts about the price. They are decoded for the record in the SDK and dropped here.
