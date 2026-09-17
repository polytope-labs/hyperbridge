# Solver-priced partial fills

`fillOrderAtRate(Order, FillOptions, TokenInfo[])` supports a solver rate on each fill while preserving
both deployed legacy `fillOrder` selectors in SDK decoding and the current contract selector.
The quoted input is a maximum; output credit determines the exact cumulative escrow release.
Oversized final bids are capped without resigning, and surplus remains destination output.

Upgrade the gateway with both matching modules, deploy the updated SolverAccount, and redelegate
rate-bidding solvers before enabling their rate quotes. `supportsRateFills()` gates SDK signing and
Simplex bidding. Old accounts remain usable for ordinary fills. No new persistent storage is added.
Pre-upgrade same-chain rounding debt remains settleable through `fillOrder` or cancellation;
explicit rate settlement of that state is rejected. Rate legs require unique token addresses until
the separate per-leg escrow upgrade is integrated.

The SDK offers opt-in automatic single-leg rate execution. Indexer decoding supports the new method
in its VM2 environment and prices signed slices using their quoted input take. Update external
orderbook and signing-policy consumers before enabling rate advertisements; their committed-order
hashing must remain unchanged. Deployment addresses and governance transactions are release inputs,
not values inferred by this change.

Sentry's rate selector policy also requires the configured live delegation and gateway/account
capabilities for phantom bids. Its existing Permit2 paymaster incompatibility remains a separate
rollout dependency; use a supported funding policy until that integration is updated.
