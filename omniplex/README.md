# Omniplex

Omniplex is Hyperbridge's indexer, built on [rindexer](https://github.com/joshstevens19/rindexer).
It writes the Postgres `app` tables that omnihedron serves. So it replaces the SubQuery indexer in
`sdk/packages/indexer` without changing the GraphQL API.

```
cargo build -p omniplex
omniplex --config omniplex/config/mainnet.toml --chains bsc-mainnet
```

The [feature note](../docs/ai/changelog/2026-11-03-rindexer-indexer.md) covers how omniplex runs,
its environment variables, and every deliberate difference from SubQuery's output.

## Cutting over from SubQuery

Omniplex takes over SubQuery's data instead of indexing from scratch. SubQuery keeps serving the
API until omniplex is shown to produce the same data.

1. Copy SubQuery's database twice, a few hours apart, with `pg_dump` and `pg_restore`. Call them
   copy A and copy B. Nothing else may connect to either copy.
2. Import both copies:

   ```
   DATABASE_URL=<copy> omniplex --config omniplex/config/mainnet.toml import-subquery
   ```

   A Substrate chain needs `<CHAIN>_SUBSTRATE` set. On testnet, add
   `--new-chains polkadex-testnet`, because SubQuery's testnet indexer doesn't index that chain.
3. Check parity between A and B, as described below.
4. Start one omniplex process per chain on A, with no stop heights. Each chain continues from
   where its parity run stopped.
5. Point omnihedron at A, then stop SubQuery.

The imported tables belong to the role that ran the import. Grant omnihedron's role access if it
is another.

## Checking parity

This is done by hand at cutover; nothing automates it yet.

Don't diff omniplex's live database against SubQuery's live one. The two indexers are never at
the same block on every chain at the same moment, so the diff fills with rows one side hasn't
reached yet. SubQuery also keeps every version of a row, while omniplex keeps only the current
one. Instead, compare both at the same heights:

1. Read B's heights: `SELECT network, block_number FROM indexer.cursors` on B. They are the
   blocks SubQuery had reached on each chain when copy B was taken.
2. Run omniplex on A, stopping each chain at its height in B. Omniplex has no option for this
   yet. Add one that sets `end_block` beside each chain's `start_block` in the contract details
   `manifest::build` writes. With an end block, rindexer indexes up to that block and doesn't
   go live. Replaying hours of blocks reads chain state at those blocks, so Ethereum and
   Arbitrum need archive RPCs.
3. Wait until rindexer's progress for every chain reaches its height in B. That is
   `last_synced_block` in the chain's `rindexer_internal.omniplex_chain__…` table. Then diff A
   and B table by table, matching rows by `id`.
4. Expect only these differences:
   - The deliberate differences in the feature note's "Parity with SubQuery" section, in rows
     written after copy A.
   - Values that several chains' processes overwrite. Their final value depends on which
     process wrote last.
   - Tables written by periodic jobs, such as token supply and vault snapshots. Omniplex runs
     these jobs after each batch of blocks, while SubQuery ran them on every block.

   Anything else is a bug. Fix it, and repeat with fresh copies, before SubQuery stops.
5. SubQuery indexes Ethereum, Arbitrum and Base before their blocks are final. So copy A can end
   on a block that a reorg later replaced. The diff then shows rows from the replaced block;
   take fresh copies.
