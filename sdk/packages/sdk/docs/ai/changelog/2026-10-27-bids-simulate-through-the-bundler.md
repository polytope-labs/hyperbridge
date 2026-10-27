# 2026-10-27 — Bids simulate through the bundler, on EntryPoint v0.9 only

`BidImpl.simulate()` used to run the bid's calldata as an `eth_call` from the solver account over
the destination chain's RPC. That call skipped what the bundler checks on submission, such as the
nonce and the paymaster. A bid with a spent nonce or an expired paymaster permit passed simulation
and then failed at `execute()`.

`simulate()` now sends the solver's UserOperation, as signed, to the bundler's
`eth_estimateUserOperationGas` on EntryPoint v0.9. The third parameter is the gateway's
selection-off override in the bundler's shape, `{ [gateway]: { stateDiff: { [slot 5]: value } } }`.
The bundler checks the nonce and the paymaster, then runs the calldata once at the bid's signed
`callGasLimit`, so a fill that reverts or runs out of gas fails simulation too.

The session's selection is not signed or sent. The bundler simulates the operation with its
`preVerificationGas` and fees changed, so the userOpHash inside the simulation is not the one a
selection signs. Without the selection, `SolverAccount.validateUserOp` reports a signature failure
before its own checks, which the bundler tolerates while estimating. Those checks, such as the nonce
key that binds the bid to the order's session, run only on submission.

No native value is forwarded. On the EntryPoint path the account pays native outputs and the
dispatch fee from its own balance.

Any bundler error fails the simulation as `Simulation failed: <reason>`, with the
`BundlerRpcError` as its `cause`. Validation failures (`AA25`, `AA33`, …) come back as `-32500` or
`-32501`, and an execution revert as `-32521`. A transient failure is retried with
`retryTransientBundlerErrors` first, as gas estimates are. `simulate()` now needs `bundlerUrl`,
which `execute()` and `BidManager.selectAndExecuteBest` already required.

Bids are routed on EntryPoint v0.9 only. Every gateway on a chain with a Hyperbridge bundler selects
by userOpHash, which pairs only with v0.9 bids, so the v0.8 route could no longer be reached there.
`BidImpl` now checks that the solver signed the bid for v0.9 and that the gateway selects by
userOpHash. A bid that fails either check is refused before any bundler call. The session key
always signs `SelectSolver(bytes32 commitment,bytes32 userOpHash)`. Chains whose configuration
still names EntryPoint v0.8 have no Hyperbridge bundler, and their bids are now refused.

`refreshSelectionFormat`, which re-read a gateway's format when a v0.8 bid met an upgraded gateway,
is removed, so `readSelectionFormat` keeps each gateway's format for the life of the process.
`KNOWN_ENTRY_POINTS` is still exported, but bids no longer use it.
