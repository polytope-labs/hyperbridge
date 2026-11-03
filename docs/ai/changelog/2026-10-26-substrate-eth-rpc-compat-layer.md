# 2026-10-26 — substrate-eth-rpc: Ethereum JSON-RPC over Substrate chains

`modules/utils/substrate-eth-rpc` answers the Ethereum JSON-RPC calls rindexer makes,
using a Substrate chain as the source. This is the first step of moving the indexer to
rindexer (polytope-labs/hyperbridge#1439). The crate README is the reference for
configuration, the log shape, the mapped events and the error codes.

## What it serves

- `eth_chainId`, `net_version`, `eth_blockNumber`, `eth_getBlockByNumber`,
  `eth_getBlockByHash`, `eth_getLogs` and `eth_call`. Every other method is `-32601`.
- Finalized blocks only. `latest`, `safe` and `pending` resolve to the finalized head.
  Above it, `eth_getBlockBy*` answers `null` and `eth_call` answers `block <n> not found`.
- Pallet events become logs. Each pallet is a contract at
  `keccak256("substrate-pallet:" ++ key)[12..]`, and each event has a Solidity-style
  signature. 18 events across `ismp`, `consensusIncentives`, `balances`, `relayer`,
  `collatorManager`, `bandwidth` and `intentsCoprocessor` are mapped in `src/mapping/`.
  `abis/` is generated from that table and pinned by `tests/abi_golden.rs`.
- `eth_call` to `0x401e375f643e635ef3b498cedbc47931d2474364` reads raw storage with
  `getStorage(bytes)` and `getChildStorage(bytes,bytes)`.

## Two ways to run it

- In the Hyperbridge node, on by default; `--no-eth-rpc` turns it off. This is the
  deployment for Nexus and Gargantua. The `eth_*` methods are merged into the node's own
  RPC module and read the node's client directly. `eth_chainId` is
  `1_000_000_000 + para_id`.
- Indexing needs an archive node: `eth_getLogs` reads `System.Events` and
  `Timestamp.Now` from each block's state, and extrinsic hashes from its body. A node
  without `--state-pruning` and `--blocks-pruning` set to `archive` or
  `archive-canonical` still serves the facade, but logs a warning at startup and fails
  ranges whose state or bodies it pruned. `--state-pruning` must be passed on every
  start, since Substrate otherwise reads the mode from the database and the node can't
  check it.
- As a standalone binary serving several chains by URL path over their Substrate RPC.
  This is for chains whose node we don't run. The binary also carries the `emit-abi`
  and `record-fixtures` tools.

## Constraints

- `eth_getLogs` is all or nothing. A block that fails to fetch or decode fails the whole
  range. Ranges over `max_block_range`, or slower than `range_deadline_ms`, answer with
  the `this block range should work: [a, b]` hint rindexer retries with.
- Block `N`'s events are decoded with block `N-1`'s metadata, so upgrade blocks decode.
  A renamed or added event field makes that event unmapped until the table is updated.
- A new variant or field in this repo's `ismp::host::StateMachine` is a compile error in
  `known_variants` (`src/abi.rs`), a reminder to decode it. Until it is decoded, blocks
  with that variant fail `eth_getLogs`.
- Indexed fields must be word-sized, `bytes` or `string`. Tuples cannot be indexed.
- `upstream_concurrency` (in-node: `--eth-rpc-concurrency`) bounds blocks being fetched
  and decoded at once. Decoding, and in-node block bodies, storage and runtime calls, run
  on tokio's blocking pool.

## Not included

- The rindexer handler port, GraphQL serving and retiring SubQuery.
- `eth_getTransactionByHash`, which the `BidPlaced` handler needs for call arguments.
- `messagingFees.FeeRewarded`, which does not exist in the runtime.
- Fixtures for the other mainnet Substrate chains (Cere, Argon, Hydration, zkVerify).
- Seven mapped events have never been emitted on Nexus or Gargantua and are covered by
  unit tests only: `GetRequestTimeoutHandled`, `AccumulateFees`, `Withdraw`,
  `BandwidthConsumed`, `SubscriptionEvicted`, `ForceCredited` and `TierSet`.
