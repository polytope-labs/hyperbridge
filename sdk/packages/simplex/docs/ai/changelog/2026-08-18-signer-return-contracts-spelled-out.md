# 2026-08-18 — Signer return contracts spelled out

The docs showed `Promise<HexString>` twice and `Promise<Signature>` once without saying that the three mean different things: `signTypedData` returns a bare 65-byte `r ‖ s ‖ v` signature (`v` 27/28, the `eth_signTypedData_v4` form), `signAuthorization` returns split components with `yParity` strictly 0/1, and `signTransaction` returns the whole signed transaction as typed-envelope RLP — not a signature at all. An implementer had to reverse-engineer that from the adapters. The contracts now live in the `Signer` and `Signature` docblocks (what an IDE shows), a "Return exactly" table on both doc pages, and the sdk's `SigningAccount.signTypedData` docblock.

Files: `src/services/wallet/types.ts`, `sdk/packages/sdk/src/types/index.ts`, `docs/content/developers/sdk/{simplex,api/simplex}.mdx`.
