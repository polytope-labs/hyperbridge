# 2026-10-12 — Simplex docs document `[[chains]]` and the free RPC endpoints

The Simplex configuration page (`docs/content/developers/evm/simplex/configuration.mdx`) gains a
`[[chains]]` section, which it did not have.

- It documents `rpcUrls` (required, non-empty, one hostname per URL, quorum-read when there is more
  than one) and `bundlerUrl` (accepted and ignored). An entry has no chain ID: boot asks every
  endpoint, and the ones that answer must agree. An endpoint that cannot answer is kept for the
  quorum, and the chain fails only when none answers.
- Its "Free RPC endpoints" table lists, for BNB Smart Chain, Ethereum, Arbitrum One, Base and
  Polygon, every endpoint that passed testing. The rows marked as defaults are `FREE_RPC_URLS` in
  `sdk/packages/simplex/src/cli/init/chains.ts`, which the setup wizards and the Chains panel use.
  The other rows passed the solo test but were left out of the defaults. Endpoints whose URL
  carries an API key are not listed.
- It says how the endpoints were tested, how the quorum sets aside one that rate-limits (its
  `Retry-After`, up to 5 minutes) or lags (15 seconds), and which free endpoints fail outright.

The overview no longer says free RPCs will not work, or that archive access is required. It points
to the table instead. The installation, Confirmations and RPC, and troubleshooting pages link to it.
