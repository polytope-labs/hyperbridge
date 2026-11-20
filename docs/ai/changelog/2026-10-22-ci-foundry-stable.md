# CI on Foundry stable

`evm.yml` and `test-solver-inventory.yml` now install Foundry `stable` instead of
v1.7.1. The pin worked around Foundry v1.8.0's `anvil_nodeInfo` probe, which broke
forks against RPC providers that reject that call. Foundry v1.8.1 made the probe
optional.

Foundry 1.8 changed two defaults that affect this repository:

- Isolate mode runs each top-level test call as its own transaction. The solver
  selection tests in `SolverAccountTest` and `IntentGatewayV2Test` call
  `validateUserOp` and `execute` separately. They rely on the transient-storage
  selection surviving between the two calls, as it does inside one EntryPoint
  `handleOps` transaction. `evm/foundry.toml` sets `isolate = false` to keep that
  behavior.
- Dynamic test linking rewrites `new X()` in test sources into cheatcode deploys.
  Test-contract artifacts in `evm/out` therefore differ from the source. The Rust
  revm tests in `evm/tests/rust` deploy some of these artifacts without cheatcodes.
  Code they call on a test contract must not create contracts with `new`.
  Today they only call pure helpers on `BeefyConsensusClientTest`.
