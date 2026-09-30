# 2026-09-08 — Permit2 `SignatureTransfer` is the only authorization simplex signs

Chosen: every sponsored UserOp carries a per-op Permit2 `PermitTransferFrom` signature, including
on tokens that implement EIP-2612.

Alternatives considered: keeping the 2612 branch for permit-capable tokens (the status quo), or
keeping it only as a zero-native bootstrap path for a fresh solver.

Why: 2612 nonces are one sequential counter per owner, so two permits signed for the same solver
carry the same nonce and only one survives. Nothing in simplex reused an allowance either — mode
`0x01` was retired from the contract, so `buildPermitMode` signed a fresh permit per op and burned
a counter slot each time. Permit2's unordered bitmap gives each op an independent nonce for the
same cost, which is what lets ops on one chain stop running single-file. Two authorization paths
for the same paymaster also meant two gas profiles, two sets of constants and a `version()` probe
whose transport errors had to be classified — all of which is now one path.

The bootstrap variant was the tempting one to keep: 2612 is the only mode that needs no
`approve(Permit2, max)`, so it let a solver with zero native and some USDC delegate and start
filling. It loses anyway. Keeping it means keeping the whole branch, the probe and the second gas
limit for a case that arises once per chain in an account's lifetime, and the batched approve in
`DelegationService.setupDelegation` already folds that approval into the delegation transaction the
solver has to send. The cost is one-time native dust on a genuinely new chain, stated in the CLI's
setup guidance.

Dropping the Circle paymaster is downstream of this, not a separate decision: it accepts EIP-2612
permits and nothing else. Its one remaining edge was Optimism, where the SDK registry has a
`CirclePaymaster` and no `SimplexPaymaster`; that chain now pays native until a Simplex paymaster
is deployed on it, which is the same fallback every unconfigured chain uses.
