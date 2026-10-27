# 2026-10-27 — Bids simulate through the bundler

`BidImpl.simulate()` used to run the bid's calldata as an `eth_call` from the solver account over
the destination chain's RPC. That call skipped what the bundler checks on submission: the nonce,
the account's validation, the paymaster, and which EntryPoint the account accepts. A bid with a
spent nonce or an expired paymaster permit passed simulation and then failed at `execute()`.

`simulate()` now sends the solver's UserOperation, as signed, to the bundler's
`eth_estimateUserOperationGas` on the EntryPoint `route()` found for the bid. The third parameter
is the gateway's selection-off override in the bundler's shape,
`{ [gateway]: { stateDiff: { [slot 5]: value } } }`. The session's selection is not signed or sent,
because the bundler estimates under its own gas limits, which change the userOpHash a selection
signs.

No native value is forwarded. On the EntryPoint path the account pays native outputs and the
dispatch fee from its own balance.

Any bundler error fails the simulation as `Simulation failed: <reason>`, with the
`BundlerRpcError` as its `cause`. Code `-32500` is a validation failure (`AA25`, `AA33`, …) and
`-32521` an execution revert. A transient failure is retried with `retryTransientBundlerErrors`
first, as gas estimates are. `simulate()` now needs `bundlerUrl`, which `execute()` and
`BidManager.selectAndExecuteBest` already required.
