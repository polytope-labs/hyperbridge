# Solver links

`OperatorMarkets` shows "Get link" at the top of an FX market's sheet. `SolverLinkDialog` calls
`planSolverLink(strategy, chainId, status.addresses.evm, name)`: it takes the first configured
ask as the token0 → token1 rate and the first bid as `reverse_rate` (both token1 per token0, the
app's unit), or, for a bid-only market, sells token1 → token0 at the reciprocal bid; `buildSolverLink`
writes the app's `/swap?wl=1&wlv=1&…` query. Copy goes through `navigator.clipboard` with a toast.
