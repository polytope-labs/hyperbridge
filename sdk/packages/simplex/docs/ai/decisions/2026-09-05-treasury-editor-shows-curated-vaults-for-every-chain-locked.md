# 2026-09-05 — Treasury editor shows curated vaults for every chain, locked when the chain is off

Chosen: the operator `/api/config` catalog and the wizard's Treasury step list the registry's Aave
stata (and other curated) vaults for every chain on the running network, and rows whose chain is
not enabled render locked with a hint pointing at the chain editor. The maintainer asked to see the
stata tokens for every chain in the Vault treasury section so operators discover what exists before
enabling a chain.

Alternatives rejected: making locked rows selectable would let a save reach `vaultPreflight`, which
hydrates each vault through `ChainClientManager.getPublicClient` and fails for a chain with no RPC,
so the row would error on save instead of explaining up front; listing both networks in the
operator catalog was rejected because a filler runs one network and testnet vaults on a mainnet
dashboard are noise; hiding disabled-chain rows (the previous behaviour) hides the catalog.
