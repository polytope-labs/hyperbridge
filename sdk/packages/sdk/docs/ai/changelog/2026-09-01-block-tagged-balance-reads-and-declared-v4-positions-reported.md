# 2026-09-01 — Block-tagged balance reads, and declared V4 positions reported out of the aggregation (#1159)

The indexer refreshes a solver's liquidity on every event that moves it, not only when a phantom bid window
closes. Three things it needs were locked inside `phantom-aggregation.ts`.

`getTotalSolverBalance` is now exported and takes a `blockTag`. It is the definition of "a solver's balance" —
raw ERC-20 plus every configured ERC-4626 vault's `maxWithdraw` — and a per-event re-read has to use exactly it:
simplex funds fills straight out of a vault inside the fill transaction, so a wallet-only `balanceOf` misses such
a fill entirely. The block tag is what lets a re-read record the balance as of the event rather than stamping
today's balance onto a historical row. `memoizedSolverBalance` takes a per-chain `blockTags` map for the same
reason — block numbers are per chain, so an event handler can pin its own chain and leave the rest of the sweep
at the head, where a number from another chain would mean nothing.

`readV4Position` is exported (now taking a params object, with a `blockTag`), `positionAmountOfToken` is
re-exported from `intents-helpers`, and `aggregatePhantomBids` reports the positions it verified as
`PhantomAggregation.positions` — solver, chain, tokenId, after the ownership check — alongside `solvers`, every
solver whose bid passed verification. `legs`, `lpBalances` and `positions` are each filtered by what a solver
turned out to hold or declare, so `solvers` is the only complete answer to "who bid this window", which is what
a consumer reconciling per-solver state needs. A bid is the only place a
Uniswap V4 position is ever named, so without this a consumer cannot re-value one between windows; and carrying
the last window's value forward is worse than not seeing it, because simplex funds fills from these positions
and such a fill drains the position while wallet and vault balances barely move.

Files: `src/protocols/intents/phantom-aggregation.ts`, `src/intents-helpers.ts`, `src/tests/phantomAggregation.test.ts`.
