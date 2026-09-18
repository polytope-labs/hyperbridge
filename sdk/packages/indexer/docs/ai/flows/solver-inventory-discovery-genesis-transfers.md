# Solver inventory (fill and watchlist discovery, genesis read, Transfer events, head)

Verified 2026-09-14 by unit tests against a mocked store and provider (`solverInventory.service.test.ts`,
`solverWatchlist.service.test.ts`, `multicall.test.ts`), and by generating the mainnet manifests and parsing every
one. The ordering fact it rests on — SubQuery's Ethereum indexer runs a block's block handlers before its log handlers — is the one
`YieldVaultService.snapshotChain` already depends on.

Each chain's EVM node maintains, for every solver it tracks, one `SolverInventory` row per supported token (the
`yieldVaults` keys of the chain's config), one `SolverVaultShares` row per supported vault and one
`SolverDelegation` row. It is the only writer of all of them. The Hyperbridge node's only part is queueing
watchlist requests.

**Discovery**

1. `handleOrderFilledEventV3` and `handlePartialFilledEventV3` call `discoverSolverFromFill` after recording the
   fill. On a chain with supported tokens, a filler with no `TrackedSolver` row `{chain}-{filler}` gets one,
   `PENDING` and `FILL`, carrying the fill's transaction hash. It reads nothing on chain.
2. On the Hyperbridge node, `handleSolverWatchlistPoll` runs every block, from inside the
   `enableSolverDiscovery` block of the substrate template, and calls `pollSolverWatchlist`:
   - It does nothing without `HYPERFX_WATCHLIST_URL`, or when the block has no timestamp or trails wall clock by
     more than 300 s. Production indexes finalized blocks, which on Nexus trail wall clock by
     roughly 40–60 s, so the bound has to cover that lag plus the node's own indexing delay.
   - Otherwise it GETs the URL with `If-None-Match` and a 5 s timeout. A `304` ends the poll. So do a transport
     error, a non-2xx answer or a body not shaped `{ chains: [{ chain, solvers: [{ address }] }] }`; each logs a
     warning and never throws.
   - Chains without supported tokens are dropped. Addresses are lowercased, validated and de-duplicated, then
     capped at 5 000 per chain with a warning.
   - Per chain, the requests already queued are read by field query. Unseen solvers become
     `SolverDiscoveryRequest` rows with `version` = the chain's `SolverWatchlist.version` + 1, and the
     `SolverWatchlist` row is rewritten with that version.
   - The response's ETag is kept only after every chain has been queued.
3. On the EVM node, `handleSolverInventoryBlock` calls `indexSolverInventoryBlock`, which returns at once on a
   chain with no supported tokens. First, `consumeWatchlist`:
   - It reads the chain's `SolverWatchlist` by field query, since the row is written by another node, and its own
     `SolverWatchlistCursor`. If the watchlist's version is not ahead of the cursor, it stops.
   - Otherwise it reads each unconsumed version's requests by (chain, version). It reads all of the chain's
     requests instead when there is no cursor, or when more than 20 versions are unconsumed.
   - Each requested solver without a `TrackedSolver` row becomes `PENDING` and `WATCHLIST`. Then the cursor
     advances.

**Genesis**

4. `seedPendingSolvers` takes up to 20 of the chain's `PENDING` solvers in id order and reads all of them with
   `readSolvers`, pinned to the block by SubQuery's provider:
   - one `readContracts` batch of every solver's `balanceOf`, for every supported token and vault, alongside each
     solver's `getCode`;
   - a second batch of `convertToAssets` for every non-zero vault balance.
   
   `readContracts` sends a batch as Multicall3 `aggregate3` calls of at most 250 reads, each allowed to fail. It
   calls each read directly instead when there is only one, or when Multicall3 has no code on the chain, at most
   `MAX_CONCURRENT_READS` in flight; a chain Multicall3 was found on is remembered for the process, an absence is
   probed again. A read that reverts or cannot be decoded fails only its own solver. The
   solver logs a warning and stays `PENDING`, nothing is written for it, a later block reads it again, and the
   other solvers are applied. A failed `aggregate3` call logs a warning and ends the step.
5. `applyReading` writes:
   - the `SolverDelegation` row. `delegated` is true only for exactly `0xef0100 ‖ a known SolverAccount`;
     `delegate` is set for any designator.
   - one `SolverVaultShares` row per vault;
   - one `SolverInventory` row per supported token, zero balances included.
   
   Every row is positioned at (block, `AFTER_EVERY_LOG`). The `TrackedSolver` becomes `TRACKED` with
   `genesisBlock` and both refresh clocks set, and joins the process's in-memory tracked set.

**Events**

6. Every supported token has its own ERC-20 datasource running `handleSolverTokenTransferEvent`, which calls
   `applyTokenTransfer`:
   - Self transfers and zero-value transfers end there.
   - A side not in the in-memory tracked set is dropped with no store read and no RPC. The set is loaded by one
     paged field query, the first time it is needed.
   - For a tracked side, the transfer applies only if its (block, log index) comes after the `SolverInventory`
     row's position. Only then is the block timestamp fetched.
   - Applying it moves `wallet` by the value (clamped at zero, with an error logged), recomputes
     `balance = wallet + vaults`, moves the position to the log, and moves `observedAt` forward only.
7. `handleVaultTransferEvent` first calls `applyVaultShareTransfer` for every share Transfer, mints and burns
   included, then carries on to the yield ledger as before.
   - The same filters and position rule apply, against `SolverVaultShares`.
   - The new share balance is valued with `convertToAssets` at the block; a failure propagates, as the ledger's
     own reads do.
   - `refoldVaults` sums the token's vault positions into `SolverInventory.vaults`, `vaultShares` and `balance`.

Why the read and the events agree: a read pinned to block N sees N's end state, and the block handler runs before
N's logs. A fill-discovered solver is still `PENDING` for the rest of its fill's block, so none of that block's
logs apply to it, and its genesis read in the next block includes them. Any row read at N skips N's logs by
position. Replaying an applied log is skipped by the same comparison.

**Head and refresh**

8. `advanceHead` does nothing while the chain's `SolverInventoryHead` is less than 30 s of block time old.
   Otherwise it runs `refreshPage` over 25 `TRACKED` solvers from `refreshOffset`:
   - A solver whose `reconciledAt` is a day old is reconciled. `readSolvers` reads every such solver on the page
     as in step 4, and its valuation batch also values each vault's drift from the tracked position. Then
     `applyReading` records `lastReconciledDrift` as read minus tracked. The drift counts the wallet plus that
     share drift, and is logged when non-zero. Reconciliation also re-checks delegation.
   - A solver whose `revaluedAt` is an hour old has its vault shares revalued and those tokens' `refreshedAt` set.
     `readRevaluations` values every such solver's positions in one batch, concurrently with the reconciliations'
     reads. The wallet is not re-read.
   - A solver with a failed read is skipped with a warning, and the offset still advances; the solver stays due
     and is read again when the cycle returns to its page. A failed `aggregate3` call ends the pass and keeps the
     offset, so the page is retried on the next advance.
   
   The head then records the block, its time, and the offset for the next pass.

What a consumer reads:
- Every `SolverInventory` row on a chain is current as of `SolverInventoryHead.blockNumber`.
- A row's `observedAt` is when its balance last changed, and `refreshedAt` is when a read last confirmed it.
- `SolverDelegation` is only as fresh as its `refreshedAt`.

Store facts this depends on:
- Every field used in a `getByFields` filter carries `@index`: `TrackedSolver.chain` and `status`,
  `SolverDiscoveryRequest.chain` and `version`, and `SolverWatchlist.chain`.
- The only cross-node reads, `SolverWatchlist` and `SolverDiscoveryRequest`, are field queries.
- A node sees only the rows whose historical range contains its own block time, so an EVM node picks up a request
  once its chain has caught up with the Hyperbridge block that queued it.
