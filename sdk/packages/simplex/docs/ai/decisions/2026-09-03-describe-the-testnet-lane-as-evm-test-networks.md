# 2026-09-03 — Describe the testnet lane as EVM test networks

Chosen: the interactive CLI calls its `testnet` bucket “EVM test networks.” The catalog groups
Ethereum Sepolia, Arbitrum Sepolia, Base Sepolia, Polygon Amoy, and BSC Chapel under `testnet`; only
the first three are Sepolia-family networks. The desktop and browser setup wizard is mainnet-only
and no longer exposes this bucket.

Alternative rejected: “Sepolia-family networks” was narrower than the actual catalog and could make
operators incorrectly assume Polygon Amoy or BSC Chapel are Sepolia deployments.
