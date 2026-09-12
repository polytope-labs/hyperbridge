# 2026-09-03 — Display Hyperbridge accounts in Polkadot's unified format

Configured the shared Substrate keyring to encode every derived account with Polkadot's unified
SS58 prefix. Setup, review, operator status, copied addresses, balance snapshots, and logs now all
receive the same unified account string without component-specific conversion.

Files: `src/services/substrate-key.ts`, `src/tests/balance-provider.test.ts`,
`docs/ai/{ChangeLog,Decisions}.md`.
