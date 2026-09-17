# 2026-09-17 — The solver inventory E2E fakes the orderbook and forks Base

Solver inventory is what the HyperFX orderbook validates orders against, and every unit test for it
runs against a mocked store and a mocked provider. Nothing proved that a real subql node, reading a
real chain, turns a real watchlist response into the rows the orderbook queries.

**A fake orderbook, not a real one.** The E2E owns the solver list, so the assertions can be exact:
these three addresses, these balances, this delegation. Running the real orderbook would need its
database, its config and live orders, and would still assert against whatever solvers happened to
exist. The response shape is not invented — it was read off `crates/server/src/app.rs` in
hyperfx-orderbook (`Watchlist` / `WatchedChain` / `WatchedSolver`), with the ETag and `304` behaviour
copied with it. That contract is the thing this test exists to protect.

**Balances written into the token's storage slot, not transferred from a whale.** The slot is already
config (`tokenSlots`), and a storage write emits no `Transfer`. That is what makes the assertions
unambiguous: the genesis read is the only way those balances could be known, and the single real
Transfer afterwards the only way the rows could then move.

**Delegation as the EIP-7702 designator itself.** `anvil_setCode` with `0xef0100 ‖ address` is exactly
what the chain holds after an authorization, so `parseDelegation` is exercised rather than simulated.
Three solvers cover the three outcomes: a known SolverAccount, a stranger (recorded, not counted), and
no code at all.

**A fork of Base, not a bare anvil.** The genesis read calls the real USDC and the real stataUSDC
vault. A stub would pass while a wrong slot, a proxy, or a vault that does not answer
`convertToAssets` would fail in production.

**The live Gargantua testnet as the Hyperbridge node.** The watchlist poll only runs on the
Hyperbridge chain, and only on blocks within 120 s of wall clock. A simnode would do it, but it costs
a full Rust build in CI; Gargantua produces live blocks and its endpoint is already a CI secret. The
trade is a dependency on a public testnet, acceptable because the poll needs nothing from it but
recent blocks. Rejected: discovering the solvers by fill instead, which needs no Hyperbridge node at
all — it would test the other discovery path and leave the orderbook endpoint, the point of the
exercise, unexercised.

**A heartbeat transaction per block**, rather than adapting the indexer. SubQuery runs block handlers
only on blocks it treats as full, and a block with no transactions is treated as light, so an idle
fork would never run the handler that consumes the watchlist and seeds solvers. One self-transfer per
block reproduces what every live chain already provides.
- Rejected: a `modulo: 1` filter on the handler. The filter is only evaluated once a block is already
  being treated as full, so it changes nothing.
- Rejected: automine, a block per transaction. It also guarantees full blocks, but makes block
  production depend entirely on the heartbeat, so a stalled sender stops the chain rather than
  skipping a block.
- Rejected: changing the indexer to tolerate empty blocks. It behaves correctly on any chain whose
  blocks carry transactions.

**The Transfer is sent from the undelegated solver.** EIP-3607 rejects transactions from an account
carrying code, which the two delegated solvers do. Both sides are tracked, so one log still moves two
rows in opposite directions, and the untouched third solver's row must not move at all.
