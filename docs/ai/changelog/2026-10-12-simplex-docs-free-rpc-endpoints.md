# 2026-10-12 — Simplex docs list the free RPC endpoints

The Simplex operator guide has a new page, `rpc-endpoints.mdx` ("Free RPC endpoints"), placed after
Confirmations and RPC in `meta.json`.

- For BNB Smart Chain, Ethereum, Arbitrum One, Base and Polygon, it gives the defaults as a
  `[[chains]]` block. These are the same lists as `FREE_RPC_URLS` in
  `sdk/packages/simplex/src/cli/init/chains.ts`, which the setup wizards and the Chains panel use.
- Each chain also lists endpoints that passed the solo test but were left out of the defaults, and
  endpoints to avoid, with the reason for each.
- The page says how the endpoints were tested, what an endpoint has to serve, and how the quorum
  sets aside one that rate-limits (its `Retry-After`, up to 5 minutes) or lags (15 seconds).
- It leaves out endpoints whose URL carries an API key.

The overview no longer says free RPCs will not work, or that archive access is required. It points
to the new page instead. The installation, Confirmations and RPC, and troubleshooting pages link
to it.
