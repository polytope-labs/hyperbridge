# 2026-10-26 — substrate-eth-rpc: Ethereum JSON-RPC over Substrate chains

`modules/utils/substrate-eth-rpc` is a Rust crate that answers Ethereum JSON-RPC over
Substrate chains, so rindexer can index them (polytope-labs/hyperbridge#1439). It runs
standalone, one process serving many chains over their Substrate JSON-RPC, each at
`http://<listen>/<chain name>`, or inside the Hyperbridge node (see "In-node mode").
It serves `eth_chainId`, `net_version`, `eth_blockNumber`, `eth_getBlockByNumber`,
`eth_getBlockByHash`, `eth_getLogs` and `eth_call`; everything else is `-32601`. The
crate README documents configuration, log shape and the rindexer manifest.

## In-node mode

- The cargo feature `node` adds `ClientRpc`, a `SubstrateRpc` implementation reading
  directly from the node client (`HeaderBackend`, `BlockBackend`, `StorageProvider`,
  `CallApiAt`, `Metadata` runtime API). `ChainConfig::in_node(..)` sets
  `upstream_retries`, `upstream_backoff_ms` and `rate_limit_cooldown_ms` to 0;
  `upstream_concurrency` still bounds parallel block decodes.
- `hyperbridge --eth-rpc` (also under `simnode`) mounts the `eth_*` methods on the
  node's RPC port next to `ismp_*`, `mmr_*` and intents. Knobs (`parachain/node/src/cli.rs`):
  `--eth-rpc-max-block-range` (500), `--eth-rpc-cache-blocks` (4096), `--eth-rpc-concurrency` (8).
- `eth_chainId` is `1_000_000_000 + para_id` from the chain spec (Nexus `1000003367`,
  Gargantua `1000004009`); hasher keccak; all mapped pallets exposed. The rindexer
  manifest `rpc` is the node URL itself, for example `http://<node>:9944`.
- Requires `--state-pruning=archive --blocks-pruning=archive`; meant for dedicated RPC
  nodes, not collators, since log decoding runs on the node's RPC threads. The node's
  150 MB RPC body limit applies instead of the standalone's 32 MB.
- This is the primary deployment for Nexus and Gargantua; the standalone binary stays
  for chains whose node is run elsewhere and for `emit-abi` and `record-fixtures`.
- Validated by the simnode test `parachain/simtests/src/eth_rpc.rs` ("Simnet Tests" CI job).

## Interfaces later indexer work depends on

- Pallet addresses: `keccak256("substrate-pallet:" ++ key)[12..]`; `ismp` is
  `0xa2692e50566b4b43335b59fec7ebc738b47f8b50`. Storage target
  `keccak256("substrate-storage")[12..]` is `0x401e375f643e635ef3b498cedbc47931d2474364`.
- Event signatures, indexed fields and type rules live in `src/mapping/`, one module per
  pallet (`ismp`, `consensusIncentives`, `balances`, `relayer`, `collatorManager`,
  `bandwidth`, `intentsCoprocessor`; 18 events). `abis/<key>.abi.json` is generated from
  that table by `emit-abi` and pinned by `tests/abi_golden.rs`.
- `StateMachine` fields are strings in `ismp::host::StateMachine`'s `Display` form
  (`EVM-1`, `POLKADOT-3367`). `Option<T>` is `(bool,T)`; fieldless enums are `uint8`.
- Block `timestamp` and log `blockTimestamp` are seconds (`Timestamp.Now / 1000`).
  Millisecond values come from `eth_call` to the storage target with
  `SubstrateStorage.getStorage(bytes key)`; `getChildStorage(bytes childKey, bytes key)`
  reads child tries. Both return `abi.encode(bytes)` of the SCALE value, empty when absent.
- `logIndex` is the event's index in `System.Events`. `transactionHash` is the extrinsic
  hash under the chain's `hasher`; `Initialization` and `Finalization` events use
  `keccak256(block_hash ++ "init")` and `keccak256(block_hash ++ "final")` with
  `transactionIndex` of `extrinsic count` and `extrinsic count + 1`.
- Error contract: `eth_getLogs` is all or nothing. `-32000` with
  `this block range should work: [0x.., 0x..]` when the range exceeds `max_block_range`
  (rindexer parses it), `-32000 block <n>: ...` for a failed block, `-32602` for
  inverted ranges, `blockHash` filters and bad `eth_call` targets.

## Constraints

- Finalized blocks only; `latest`, `safe` and `pending` resolve to the finalized head.
- The Substrate endpoint, or the node itself in in-node mode, must be an archive node:
  historic `System.Events`, `Timestamp.Now`, `state_getRuntimeVersion` and metadata are
  read per block.
- Runtime upgrades: the events of block `N` are decoded with the runtime metadata of
  block `N-1` (cached per spec version) and matched by pallet, variant and field set, so
  a renamed or added field makes that shape unmapped until a new entry is added. Unmapped
  shapes are skipped, counted and logged once; a changed nested type (`StateMachine`,
  `StateMachineId`, `TierConfig`, `TierIndex`) fails the block with `-32000` until the
  mapping is updated.
- `eth_getBlockByHash` and hash-based `eth_call` answer only for finalized canonical
  blocks; any other hash is `null` or `block not found`.
- Upstream load is bounded by `upstream_concurrency` (default 8, halved for
  `rate_limit_cooldown_ms` after a rate-limit answer); transient upstream errors (rate
  limits, timeouts, lost connections, HTTP 5xx) are retried with exponential backoff.
  In-node mode has no upstream, so retries and cooldowns are off and only the
  concurrency bound remains.
- An `eth_getLogs` range that misses `range_deadline_ms` (default 60 s) answers with the
  same `this block range should work: [from, k]` hint, `k` being the end of the already
  decoded prefix, while the running decodes finish into the cache; no partial logs.

## Not included

- The rindexer handler port, GraphQL serving and SubQuery retirement.
- A Dockerfile.
- `eth_getTransactionByHash`; the `BidPlaced` handler's extrinsic call arguments need it.
- `messagingFees.FeeRewarded`, which does not exist in the runtime.
- The three-field `BidPlaced` (no `bid`), which is skipped.
- Configuration and fixtures for the other mainnet Substrate chains (Cere, Argon,
  Hydration, zkVerify); only `hyperbridge-nexus` and `gargantua` are fixtured.
- Recorded fixtures for `GetRequestTimeoutHandled`, `AccumulateFees`, `Withdraw`,
  `BandwidthConsumed`, `SubscriptionEvicted`, `ForceCredited` and `TierSet`, which have
  not been emitted on Nexus or Gargantua; unit tests with hand-built events cover them.
