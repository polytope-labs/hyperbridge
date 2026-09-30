# 2026-09-05 — Show curated vaults for every chain in the treasury editors

The dashboard's Vault treasury drawer and the wizard's Treasury step now list the registry's Aave
stata (and other curated) vaults for every chain on the running network. `UiServer.knownVaultCatalog`
returns the catalog for all `INIT_CHAINS` on that network instead of only running chains;
`VaultRowsEditor` orders enabled chains first and renders the others locked (disabled checkbox,
`data-disabled`, hint naming where to enable the chain: Chains & endpoints, or the wizard's Chains
step). Select all only covers selectable rows. The minimum-balance tooltip's paymaster note now
covers USDT as well as USDC. Added a ui-server test for the widened catalog. Cross-checked the
registry against the indexer's `yieldVaults` (identical where they overlap) and the Aave address
book: Aave v3 Base lists no USDT, so there is no Base stataUSDT to add.
Files: `src/services/server/{UiServer,dto}.ts`, `ui/src/components/VaultRowsEditor.tsx`,
`ui/src/operator/Operations.tsx`, `ui/src/wizard/steps/Treasury.tsx`, `ui/src/styles/treasury.css`,
`src/tests/ui-server.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
