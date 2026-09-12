# 2026-08-28 — Export `FILL_ORDER_V1_ABI` for consumers that cannot use `decodeFillOrder`

`decodeFillOrder` handles both `fillOrder` shapes, but it is viem-based and viem's byte handling throws inside
SubQuery's VM2 sandbox, so the indexer reimplements the decode with ethers. It was building its interface from
`FILL_ORDER_ABI` alone and therefore rejected every v1-shaped bid — which is every bid on mainnet today, since no
gateway has been upgraded past `validUntil` yet.

The legacy shape is now exported (from `fillOrderCodec`, re-exported on the `intents-helpers` sub-path) so that
decoder can share this definition instead of keeping a second copy. A duplicated ABI that drifts out of step with
this one is exactly the failure being fixed, so the constant is exported rather than re-declared downstream.

Only the constant is exported on the VM2-safe sub-path; `decodeFillOrder` itself stays off it.

Files: `src/protocols/intents/fillOrderCodec.ts`, `src/protocols/intents/index.ts`, `src/intents-helpers.ts`.
