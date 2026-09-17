# 2026-09-17 — The solver-inventory E2E fakes the orderbook and forks Base

The setting: solver inventory is what the HyperFX orderbook validates orders against, and every unit test for it
runs against a mocked store and a mocked provider. Nothing proved that a real subql node, reading a real chain,
turns a real watchlist response into the rows the orderbook queries.

**Chosen: a fake orderbook, not a real one.** The E2E owns the solver list, so the assertions can be exact —
these three addresses, these balances, this delegation. Running the real orderbook would need its database, its
config and live orders, and would still assert against whatever solvers happened to exist.
- The shape is not invented: it was read off `crates/server/src/app.rs` in hyperfx-orderbook (`Watchlist` /
  `WatchedChain` / `WatchedSolver`) and the ETag and `304` behaviour copied with it. That is the contract this test
  exists to protect, so it is the one thing the test must not mock loosely.

**Chosen: balances written into the token's storage slot, not transferred from a whale.** The slot is already
config (`tokenSlots`), and a storage write emits no `Transfer`. That is what makes the later assertions
unambiguous: the genesis read is the only way those balances could be known, and the one real Transfer afterwards
is the only way the rows could then move.

**Chosen: delegation as the EIP-7702 designator itself.** `anvil_setCode` with `0xef0100 ‖ address` is exactly what
the chain holds after an authorization, so `parseDelegation` is exercised rather than simulated. Three solvers
cover the three outcomes: a known SolverAccount, a stranger (recorded, not counted), and no code at all.

**Chosen: a fork of Base, not a bare anvil.** The genesis read calls the real USDC and the real stataUSDC vault. A
stubbed token would pass while a wrong slot, a proxy, or a vault that does not answer `convertToAssets` would fail
in production.

**Chosen: the live Gargantua testnet as the Hyperbridge node.** The watchlist poll only runs on the Hyperbridge
chain, and only on blocks within 120 s of wall clock. A simnode would do it, but it costs a full Rust build in CI;
Gargantua already produces live blocks and its endpoint is already a CI secret. The trade is a dependency on a
public testnet — acceptable because the poll needs nothing from it but recent blocks.
- Rejected: discovering the solvers by fill instead, which needs no Hyperbridge node at all. It would test the
  other discovery path and leave the orderbook endpoint — the point of the exercise — unexercised.

**Chosen: the Transfer is sent from the undelegated solver.** EIP-3607 rejects transactions from an account
carrying code, which the two delegated solvers do. Both sides of the transfer are tracked, so one log still has to
move two rows in opposite directions, and the untouched third solver's row must not move at all.

Known limits:
- **Public endpoints.** CI forks with the archive-capable `BASE_MAINNET` secret. A public Base RPC without archive
  access cannot mine against the fork, which is a local-development trap, not a CI one.
- **Timing.** Discovery waits on a Gargantua block, then a Base block, then the head advance, so the genesis
  assertion allows 15 minutes before it gives up.
