# 2026-09-14 — Solver inventory reads are batched through Multicall3

The setting: every genesis read, reconciliation and revaluation cost one request per token balance, per vault
balance and per share valuation, solver by solver. A Base solver was 12 requests, an Ethereum one 14. A genesis
burst of 20 solvers, or a refresh page of 25, took hundreds of sequential round trips inside one block handler.

**Chosen: Multicall3's `aggregate3` at its canonical address, with `allowFailure: true`.** Each read settles on its
own, so a reverting vault fails only the solvers whose reading needs it.
- Rejected: `allowFailure: false`. One bad vault would fail every solver in the batch, every block.
- Rejected: JSON-RPC batching of the individual `eth_call`s. It still costs the provider one execution per read, and
  SubQuery's batch size is node configuration this code cannot rely on.

**Chosen: two batches per pass, balances then valuations.** `convertToAssets` needs the share balance, so the second
batch cannot be folded into the first. A reconciliation's drift valuation (read minus tracked shares) joins the
second batch, which is why positions are loaded before it rather than inside `applyReading`.
- Rejected: reading one rate per vault (`convertToAssets(10^decimals)`) in the first batch and multiplying. ERC-4626
  rounding makes that differ by wei from the exact `convertToAssets(shares)` the share Transfer path writes, so the
  two paths would disagree about unchanged positions.

**Chosen: `getCode` stays one request per solver, sent concurrently with the first batch, at most
`MAX_CONCURRENT_READS` in flight.** Multicall3 has no code read. These do not coalesce: `EthereumApi.getSafeApi`
hands a mapping `nonBatchClient`, a plain `JsonRpcProvider`, on HTTP endpoints, because batched historical queries
are not routed to archive nodes everywhere. So a 20-solver genesis is two `eth_call`s plus 20 `eth_getCode`s, down
from about 240 requests.
- Rejected: a helper contract, or a deployless `eth_call` that returns `EXTCODECOPY`. The first needs deploying on
  every chain, and the second adds bytecode to maintain for one read per solver. Worth revisiting only if the
  `getCode`s, now the larger half of a genesis burst, start to hurt.

**Chosen: fall back to individual calls where Multicall3 has no code, `MAX_CONCURRENT_READS` at a time.**
`eth_getCode` on 2026-09-14 found it on every configured chain except Polkadot Asset Hub (420420419) and Paseo
(420420417). Only a positive answer is cached: code never disappears, while an absence is also what a block before
the deployment, or a node briefly serving empty code, returns — caching that would quietly drop the chain to
unbatched reads until the process restarts. A single read also goes direct, because wrapping one call saves
nothing.

**Chosen: a failed read skips its solver, not the pass.** The other solvers' reads have already been paid for in the
same batch. Before, a solver whose read always failed also stalled every pending solver after it in id order.
- **Genesis.** A skipped solver stays `PENDING`.
- **Refresh.** A skipped solver does not hold the page: the offset still advances. Such a failure is usually a
  revert that will keep failing, and holding the page would starve every solver behind it — with 25 per page, one
  bad solver froze every later page. It stays due, so the next pass over its page reads it again. A failed batch
  is the transient case, the RPC being down, and does keep the offset.

`aggregate3` calls carry at most 250 reads, well inside providers' `eth_call` gas and response limits. A genesis
burst on Ethereum is about 220 balance reads.
