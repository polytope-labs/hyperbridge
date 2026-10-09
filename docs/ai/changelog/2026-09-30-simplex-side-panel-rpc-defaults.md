# Simplex side-panel chain defaults

New chains in the operator's Chains panel start with their catalog's `defaultRpcUrls`, matching
the setup wizard. Chains without bundled endpoints start with one empty RPC field. Configured
chains retain their saved RPC endpoints, including custom chains outside the catalog.

The panel has no bundler field. Every catalog chain is saved with Hyperbridge's bundler; see
`docs/ai/changelog/2026-10-06-entrypoint-v09-end-to-end.md`. Chains without public defaults still
require an RPC URL from the operator.
