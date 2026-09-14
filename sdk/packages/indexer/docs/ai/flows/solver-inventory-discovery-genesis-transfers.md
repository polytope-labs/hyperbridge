# Solver inventory (fill and watchlist discovery, genesis read, Transfer events, head)

Verified 2026-09-14 by unit tests against a mocked store and provider (`solverInventory.service.test.ts`,
`solverWatchlist.service.test.ts`), and by generating the mainnet manifests and parsing every one. The ordering
fact it rests on — SubQuery's Ethereum indexer runs a block's block handlers before its log handlers — is the one
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
     more than 120 s.
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

4. `seedPendingSolvers` takes up to 20 of the chain's `PENDING` solvers in id order. For each, `readSolver`
   reads, pinned to the block by SubQuery's provider:
   - `getCode`;
   - every supported token's `balanceOf`;
   - every supported vault's `balanceOf` and `convertToAssets`.
   
   A failed read logs a warning and ends the step. The solver stays `PENDING`, nothing is written for it, and a
   later block reads it again.
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
   - A solver whose `reconciledAt` is a day old is reconciled: `readSolver`, then `applyReading`, which records
     `lastReconciledDrift` as read minus tracked. The drift counts the wallet plus share drift valued at the block,
     and is logged when non-zero. Reconciliation also re-checks delegation.
   - A solver whose `revaluedAt` is an hour old has its vault shares revalued and those tokens' `refreshedAt` set.
     The wallet is not re-read.
   - A failed read ends the pass and keeps the offset.
   
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
