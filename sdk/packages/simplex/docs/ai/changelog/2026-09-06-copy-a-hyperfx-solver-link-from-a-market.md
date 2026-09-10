# 2026-09-06 — Copy a HyperFX solver link from a market

Each FX market's sheet on the Overview gets a "Get link" entry that opens a dialog building the
HyperFX white-label "solver link" from the running settings: the filler's EVM address
(`status.addresses.evm`), the chosen chain, the market's first curve prices (ask as `rate` for
token0 → token1, bid as `reverse_rate`; a bid-only market flips direction with the reciprocal),
and a solver name (max 24, remembered in localStorage). The format was read from the app's bundle
(`app.hyperfx.finance/swap?wl=1&wlv=1&source&destination&from&to&rate_base&rate_quote&rate&reverse_rate&solver&solver_name`;
`from`/`to` must equal `rate_base`/`rate_quote`, path must be `/swap`). `ui/src/lib/solver-link.ts`
holds the builder and the per-market plan; `SolverLinkDialog` shows the summary and URL and copies
it. Same-asset, reference and venue-priced markets are refused with a reason. Unit-tested.
Files: `ui/src/lib/solver-link.ts`, `ui/src/components/SolverLinkDialog.tsx`,
`ui/src/operator/{OperatorMarkets,OperatorOverview}.tsx`, `ui/src/styles/operator.css`,
`src/tests/solver-link.test.ts`, `docs/ai/{ChangeLog,Flow}.md`.
