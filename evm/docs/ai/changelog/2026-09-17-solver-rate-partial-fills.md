# Solver-priced partial fills

`fillOrder(Order, FillOptions)` accepts positional input takes in the appended `FillOptions.inputs`
field. Each take is a maximum; output credit determines the exact cumulative escrow release.
Empty inputs retain order-rate settlement. Oversized final bids are capped without resigning,
and surplus remains destination output. Order commitments, proofs, messages, and storage are unchanged.

The new selector is `0x68ddf058`. SDK codecs retain the historical v1/v2 shapes for old deployments;
upgraded gateways require new calldata, including for ordinary fills. Upgrade the gateway with both
matching modules, deploy the updated SolverAccount, and redelegate participating solvers before
publishing new bids. `fillOrderSelector()` must match on the gateway, configured account implementation,
and live solver delegation. This check applies to every v3 bid, including empty inputs and phantom bids.
The account rejects stripped bid signatures for the new and both historical selectors.
Drain or requote outstanding old-selector bids around each gateway upgrade.

Pre-upgrade same-chain rounding debt remains settleable with empty inputs or cancellation;
explicit rate settlement of that state is rejected. Rate legs require unique, canonical token identifiers
under the current token-keyed storage layout.

Automatic SDK execution ranks single-leg quotes by rate. A durable pending journal retains the exact
signed operation before broadcast; restart reconciles its outcome and can rebroadcast the same operation.
Definitive first-send rejection permits the next bid, while uncertain outcomes remain pending.
Indexer decoding supports all three ABI shapes in VM2 and prices slices using their quoted input takes.

Update external orderbook and signing-policy consumers before enabling new quotes. Sentry's existing
Permit2 paymaster incompatibility remains a separate rollout dependency; use a supported funding policy.
Deployment addresses and governance transactions are release inputs.
