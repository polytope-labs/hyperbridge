# 2026-09-22 — Mainnet SolverAccount moves to the 2026-09-22 deployment; 2.8.16

Every mainnet chain config with a `SolverAccount` (ethereum, bsc, arbitrum, base, polygon, optimism,
gnosis, polkadot hub) points it at `0xd5535d4DeB17F050e52B6efda2fDe00435f39279`, the account
deployed alongside the new IntentGatewayV2 implementation on 2026-09-22 (PR #1317), replacing
`0x7cb55539d1144F62422099c3FA3405092022c88C`. The new account derives the bid nonce key from the
commitment, session key and the op's calldata, and relies on the upgraded gateway, so a solver
delegating to it can only bid on a chain whose gateway proxy runs `0x8E6c939E…2648`.

The bid-verification fixture in `phantomAggregation.test.ts` uses the same address. Simplex 0.16.5
goes with it.
