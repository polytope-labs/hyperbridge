# Solver-priced partial fills

`fillOrder(Order, FillOptions)` accepts positional input takes in the appended `FillOptions.inputs`
field. Each take is a maximum; output credit determines the cumulative escrow release. Empty inputs
retain order-rate settlement. Oversized final bids are capped without resigning, and excess output
is split as surplus. Legs may repeat canonical token addresses; escrow and progress are per leg.

The selector is `0x68ddf058`, encoded as FillOptions ABI version 3. Gateway and SolverAccount release
version 4 identify support. Before signing, the SDK checks the gateway, configured account implementation
and live solver delegation, including bids with empty inputs and phantom bids. Historical ABI versions
1 and 2 remain decodable. The account protects all three selectors against stripped bid signatures.

Upgrade the gateway and both modules atomically with `migrate(owner)`. Supported version-2 proxies
initialize ownership and shift the legacy relayer slot; current owner-layout version-3 proxies preserve
owner, pending owner, relayer and pause state. Earlier module-only implementations that also reported
version 3 are not supported predecessors. Deploy the new SolverAccount and redelegate solvers before
publishing new bids; outstanding old-selector bids need new calldata and signatures.

Token-keyed deployments must first drain escrow, fees and pending messages on every chain, then stop
placement until all chains use per-leg accounting. This is the rollout requirement of the per-leg
escrow upgrade. An already-per-leg deployment can retain its order state: old same-chain rounding debt
can finish with empty inputs or cancel, while explicit rate settlement rejects it.

Automatic SDK execution ranks single-leg quotes by rate. It persists the signed operation before
broadcast and reconciles it after restart. Definitive first-send rejection permits fallback; ambiguous
outcomes retain the same operation. Receipt attribution uses the accepted operation's EntryPoint log
interval, gateway, order commitment and solver. Simplex rounds funding-limited input takes down so
rounding cannot push the quote below the user's limit price.

Update external orderbook and signing-policy consumers before enabling quotes. Sentry's existing
Permit2 paymaster incompatibility remains a separate rollout dependency; use a supported funding policy.
