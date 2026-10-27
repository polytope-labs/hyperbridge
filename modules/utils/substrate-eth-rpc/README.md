# substrate-eth-rpc

An Ethereum JSON-RPC facade over Substrate chains. It lets EVM indexers such as
rindexer index Substrate chains: pallet events become Ethereum logs, blocks take the
Ethereum block shape, and Substrate storage is readable through `eth_call`. Only
finalized blocks are served, so a client never sees a reorg.

The facade runs in one of two ways: as a standalone binary that talks to any Substrate
node over its JSON-RPC, or inside the Hyperbridge node itself, reading the node's own
client with no RPC round trip (see "Running inside the Hyperbridge node").

## Running standalone

```sh
cargo run -p substrate-eth-rpc -- serve --config config.toml
```

`config.example.toml` is a complete example. Every `${NAME}` in `rpc` is replaced with the
environment variable `NAME`; an unset variable is a startup error.

| Field | Default | Meaning |
|---|---|---|
| `listen` | `0.0.0.0:8545` | HTTP listen address (HTTP only, no WebSocket) |
| `log` | `info` | Default tracing filter; `RUST_LOG` overrides it |
| `chains[].name` | required | URL path segment, must match `[a-z0-9-]+`, unique |
| `chains[].rpc` | required | Substrate WebSocket URL(s) (`ws://` or `wss://`), comma separated; tried in order at startup, the first reachable one is used for the process lifetime, reconnects go to it, and failover to the next URL requires a restart |
| `chains[].eth_chain_id` | required | Value returned by `eth_chainId` and `net_version`, unique |
| `chains[].hasher` | required | `keccak` or `blake2`; the chain's extrinsic hasher, used for `transactionHash` |
| `chains[].pallets` | required | Pallet keys to map (see below); an unknown key is a startup error |
| `chains[].max_block_range` | `500` | Largest `eth_getLogs` span, accepted when `toBlock - fromBlock <= max_block_range`, at least 1; set it equal to the rindexer manifest `max_block_range`, both mean `to - from <= N`; 100 to 200 for a cold sync against a slow archive node, the deadline hint below handles the rest |
| `chains[].cache_blocks` | `4096` | Decoded blocks kept in the LRU cache |
| `chains[].upstream_concurrency` | `8` | Blocks fetched from the Substrate node at once, at least 1 |
| `chains[].upstream_retries` | `5` | Retries of a transient upstream error per call, after the first attempt |
| `chains[].upstream_backoff_ms` | `250` | Delay before the first retry; doubles per retry, capped at 5 s, plus up to 25% jitter |
| `chains[].rate_limit_cooldown_ms` | `10000` | After a rate-limit answer, each block fetch holds two concurrency permits for this long, halving the load on the node |
| `chains[].range_deadline_ms` | `60000` | Time an `eth_getLogs` range may take before it answers with a hint to retry a shorter range, at least 1000; keep it below the client's HTTP timeout (rindexer: 90 s) |

Each chain is served at `http://<listen>/<name>`. Any other path returns HTTP 404. When
exactly one chain is configured it is also served at `/`. The per-path layout and the
`chains[]` table above apply to the standalone binary only.

## Running inside the Hyperbridge node

With the cargo feature `node`, the crate exposes `ClientRpc`, a `SubstrateRpc`
implementation that reads blocks, storage, runtime versions and metadata straight from
a Substrate node client (`HeaderBackend`, `BlockBackend`, `StorageProvider`,
`CallApiAt` and the `Metadata` runtime API). The `hyperbridge` binary uses it to mount
the same `eth_*` methods next to its `ismp_*`, `mmr_*` and intents RPCs:

```sh
hyperbridge --chain nexus --state-pruning=archive --blocks-pruning=archive --eth-rpc
```

The flags are accepted by both `hyperbridge` and `hyperbridge simnode`.

| Flag | Default | Meaning |
|---|---|---|
| `--eth-rpc` | off | Serve the facade on the node's RPC port |
| `--eth-rpc-max-block-range` | `500` | Largest `eth_getLogs` span, accepted when `toBlock - fromBlock <= N` |
| `--eth-rpc-cache-blocks` | `4096` | Decoded blocks kept in the LRU cache |
| `--eth-rpc-concurrency` | `8` | Parallel block decodes per `eth_getLogs` |

The methods are served on the node's normal RPC endpoint with no path, for example
`http://<node>:9944`. Chain identity comes from the chain spec: `eth_chainId` is
`1_000_000_000 + para_id` (Nexus `1000003367`, Gargantua `1000004009`), the hasher is
`keccak` and every pallet in the mapping table is exposed. Because the data comes from
the local client there are no upstream retries, backoff or rate-limit cooldowns:
`rpc`, `upstream_retries`, `upstream_backoff_ms` and `rate_limit_cooldown_ms` do not
apply; `range_deadline_ms` is fixed at 60 s. Request and response bodies are capped
by the node at 150 MB (`hyperbridge simnode` keeps the 15 MB sc-cli default), more than the
standalone's 32 MB.

The node must run with `--state-pruning=archive --blocks-pruning=archive` so historic
`System.Events`, `Timestamp.Now` and metadata are readable. Enable the flag on
dedicated RPC nodes rather than collators: `eth_getLogs` decoding is CPU work on the
node's RPC threads.

The rindexer manifest then points at the node directly:

```yaml
networks:
  - name: hyperbridge-nexus
    chain_id: 1000003367
    rpc: http://<node>:9944
    max_block_range: 500
```

This is the intended deployment for Nexus and Gargantua. The standalone binary remains
for chains whose node is run by someone else, and for `emit-abi` and `record-fixtures`.

## Supported methods

| Method | Params | Answer |
|---|---|---|
| `eth_chainId` | none | `eth_chain_id` as hex |
| `net_version` | none | `eth_chain_id` as a decimal string |
| `eth_blockNumber` | none | Finalized head, cached for 500 ms |
| `eth_getBlockByNumber` | number or tag, `fullTransactions` ignored | Block, or `null` above the finalized head |
| `eth_getBlockByHash` | hash, `fullTransactions` ignored | Block, or `null` unless the hash is a finalized canonical block |
| `eth_getLogs` | filter with `fromBlock`/`toBlock` (numbers or tags, default `finalized`), `address`, `topics` | Logs in block order |
| `eth_call` | `{to, data}` or `{to, input}`, block number, tag or hash (default `finalized`) | Storage reads, see below |

Tags `latest`, `finalized`, `safe` and `pending` all resolve to the finalized head;
`earliest` is block 0.

Block fields: `hash`, `parentHash`, `number`, `stateRoot`, `transactionsRoot` (the
extrinsics root), `timestamp` (seconds), `logsBloom` (a real bloom over the mapped logs)
and `transactions` (extrinsic hashes). `miner`, `difficulty`, `gasLimit`, `gasUsed`,
`receiptsRoot`, `mixHash`, `nonce` and `extraData` are zero or empty; `baseFeePerGas`,
`size` and `totalDifficulty` are omitted.

Not supported: `blockHash` filters in `eth_getLogs`, `eth_getTransactionReceipt`,
`eth_getTransactionByHash`, traces, subscriptions. Any other method returns `-32601`.

## How events become logs

Each configured pallet is a synthetic contract:

```
address = keccak256("substrate-pallet:" ++ key)[12..]
```

| Key | Address |
|---|---|
| `ismp` | `0xa2692e50566b4b43335b59fec7ebc738b47f8b50` |
| storage target (`keccak256("substrate-storage")[12..]`) | `0x401e375f643e635ef3b498cedbc47931d2474364` |

For every event in `System.Events` whose pallet is configured, the mapping table in
`src/mapping/` is searched for an entry with the same variant name and the same set of
field names. A match produces one log:

- `topics[0]` is `keccak256` of the Solidity signature.
- Indexed fields follow as topics (word-sized values as-is; `bytes`, `string` and tuples
  are hashed with `keccak256`).
- `data` is the ABI encoding of the non-indexed fields in signature order.
- `logIndex` is the event's index in `System.Events`, so it has gaps where events were
  not mapped.
- `transactionIndex` and `transactionHash` are the extrinsic's index and hash for
  `ApplyExtrinsic` events. `Initialization` events use index `extrinsic count` and hash
  `keccak256(block_hash ++ "init")`; `Finalization` events use `extrinsic count + 1` and
  `keccak256(block_hash ++ "final")`.
- `blockTimestamp` is `Timestamp.Now / 1000`, in seconds, the same as the block's
  `timestamp`. Handlers that need milliseconds read `Timestamp.Now` through the storage
  `eth_call` below.
- `removed` is always `false`.

An event from a configured pallet with no matching entry (unknown variant, or a variant
whose field set changed in a runtime upgrade) is skipped, counted, and logged once per
shape. Events from pallets that are not configured are ignored silently.

### Type mapping

| Substrate | Solidity |
|---|---|
| `H256`, `AccountId32` | `bytes32` |
| `[u8; 4]` | `bytes4` |
| `u64`, `u128`, `U256` | `uint64`, `uint128`, `uint256` |
| `Vec<u8>`, `BoundedVec<u8>` | `bytes` |
| `StateMachine` | `string`: `EVM-1`, `POLKADOT-3367`, `KUSAMA-<id>`, `SUBSTRATE-<id>`, `TNDRMINT-<id>`, `RELAY-<relay>-<para_id>` |
| `Option<T>` | `(bool,T)`; `None` is `(false, zero value of T)` |
| fieldless enum | `uint8` (the discriminant listed in the mapping module) |
| struct | tuple, in the field order of the mapping entry |

Newtype wrappers (`H256([u8; 32])`, `BoundedVec(Vec<u8>)`) are unwrapped before
conversion.

### Mapped events

Signatures are exact; `abis/` is generated from the same table. Parameters are
`indexed` where listed.

| Pallet | Signature | Indexed |
|---|---|---|
| `ismp` | `StateMachineUpdated((string,bytes4),uint64)` | none |
| `ismp` | `Request(string,string,uint64,bytes32)` | `commitment` |
| `ismp` | `Response(string,string,uint64,bytes32,bytes32)` | `commitment`, `reqCommitment` |
| `ismp` | `PostRequestHandled(bytes32,bytes)` | `commitment` |
| `ismp` | `GetRequestHandled(bytes32,bytes)` | `commitment` |
| `ismp` | `PostRequestTimeoutHandled(bytes32,string,string)` | `commitment` |
| `ismp` | `GetRequestTimeoutHandled(bytes32,string,string)` | `commitment` |
| `consensusIncentives` | `RelayerRewarded(bytes32,uint128,((string,bytes4),uint64))` | `relayer` |
| `balances` | `Transfer(bytes32,bytes32,uint128)` | `from`, `to` |
| `relayer` | `AccumulateFees(bytes,string,uint256)` | none |
| `relayer` | `Withdraw(bytes,bytes,string,uint256)` | none |
| `collatorManager` | `CollatorRewarded(bytes32,uint128)` | `collator` |
| `bandwidth` | `BandwidthCredited(string,bytes,string,uint8,uint128,uint64)` | none |
| `bandwidth` | `BandwidthConsumed(string,bytes,uint128,uint128)` | none |
| `bandwidth` | `SubscriptionEvicted(string,bytes,uint8,uint128)` | none |
| `bandwidth` | `ForceCredited(string,bytes,uint8,uint128,uint64)` | none |
| `bandwidth` | `TierSet(uint8,(bool,(uint128,uint64)))` | none |
| `intentsCoprocessor` | `BidPlaced(bytes32,bytes32,bytes32,uint128)` | `filler`, `commitment` |

The `bandwidth` tier `uint8` is `TierOne` = 1, `TierTwo` = 2, `TierThree` = 3,
`TierFour` = 4. `BidPlaced` is mapped only in its four-field form (`filler`,
`commitment`, `bid`, `deposit`); the three-field form without `bid` is skipped.

Parameter names in `abis/` are camelCase versions of the runtime field names, with
`bytes` renamed to `byteCount`, `address` to `relayer` and `beneficiary_address` to
`beneficiary`.

## Storage `eth_call`

The storage target `0x401e375f643e635ef3b498cedbc47931d2474364` implements:

```solidity
interface SubstrateStorage {
    function getStorage(bytes key) external view returns (bytes value);
    function getChildStorage(bytes childKey, bytes key) external view returns (bytes value);
}
```

`key` is the raw storage key, for example `twox128("Timestamp") ++ twox128("Now")`. The
return value is `abi.encode(bytes)` of the SCALE-encoded storage value at the requested
block, or of empty bytes when the key is absent. `getStorage` maps to `state_getStorage`
and `getChildStorage` to `childstate_getStorage`. The block parameter may be a number,
a tag or a block hash; a hash that is not a finalized canonical block is `block not
found`. Any other `to` address returns `-32602 only the storage address is
callable`; an unknown selector returns `-32602 unknown storage call: ...`.

## Errors

`eth_getLogs` is all or nothing: if any block in the range cannot be fetched or decoded,
the whole request fails and no partial result is returned.

| Code | Message | When |
|---|---|---|
| `-32000` | `this block range should work: [0x64, 0x65]` | `toBlock - fromBlock` larger than `max_block_range` (the hint ends at `fromBlock + max_block_range`), or the range missed `range_deadline_ms`; rindexer parses this hint and retries with the suggested range |
| `-32000` | `toBlock 0x67 is beyond finalized head 0x66` | Range ends after the finalized head |
| `-32000` | `block <n>: <cause>` | A block in the range failed to fetch or decode |
| `-32000` | `block <n> not found` | `eth_call` at a block above the finalized head or at a hash that is not a finalized canonical block |
| `-32602` | `fromBlock <a> is after toBlock <b>` | Inverted range |
| `-32602` | `blockHash filter not supported` | `eth_getLogs` with `blockHash` |
| `-32602` | parameter errors | Bad `eth_call` target or selector, malformed params |

When a range misses `range_deadline_ms` the hint is `[fromBlock, k]`, where `k` is the
last block of the longest already decoded prefix, or `[fromBlock, fromBlock]` when the
first block is not decoded yet. The decodes that were running keep going and land in the
cache, so the retried range is served from cache and the client advances. No partial
logs are ever returned.

Transient upstream errors (rate limits such as `-32999`, `429` or `Too Many Requests`,
timeouts, lost or reconnecting connections, HTTP 5xx) are retried per call up to
`upstream_retries` times with exponential backoff from `upstream_backoff_ms`, capped at
5 s, each retry logged at `warn`. Any other error fails the block, and so the range, at
once. A rate-limit answer also starts a `rate_limit_cooldown_ms` cooldown during which the
effective `upstream_concurrency` is halved.

Every request logs its method, chain, resolved block or range, outcome and elapsed time
at `debug` (`substrate_eth_rpc=debug`); a failed `eth_getLogs` is logged at `warn`.

The Substrate endpoint must be an archive node. Every block read fetches historic
`System.Events` and `Timestamp.Now` through `state_getStorage`, the runtime version
of the parent block through `state_getRuntimeVersion`, and, once per spec version, the
metadata through `state_call Metadata_metadata_at_version` (v15) with
`state_getMetadata` as fallback.

### Runtime upgrades

The events of block `N` are decoded with the runtime metadata of block `N-1`, the
runtime that produced them, so blocks on either side of an upgrade keep decoding. An
upgrade that renames or adds a top-level field of a mapped event makes that event
unmapped: it is skipped and counted until the mapping table gains the new shape. An
upgrade that changes a nested type the mapping depends on (`StateMachine` variants,
`StateMachineId`, `TierConfig`, `TierIndex`) makes every block containing that event
fail with `-32000` until the mapping table is updated and the service redeployed.

## rindexer manifest

```yaml
networks:
  - name: hyperbridge-nexus
    chain_id: 1000003367
    rpc: http://127.0.0.1:8545/hyperbridge-nexus
    max_block_range: 500

contracts:
  - name: Ismp
    details:
      - network: hyperbridge-nexus
        address: "0xa2692e50566b4b43335b59fec7ebc738b47f8b50"
        start_block: "0"
    abi: ./abis/ismp.abi.json
    include_events:
      - StateMachineUpdated
      - Request
      - Response
      - PostRequestHandled
      - GetRequestHandled
      - PostRequestTimeoutHandled
      - GetRequestTimeoutHandled
```

`chain_id` must equal the chain's `eth_chain_id` (standalone) or `1_000_000_000 +
para_id` (in-node, where `rpc` is the node's own RPC URL). `disable_logs_bloom_checks` is not
needed because blocks carry a real `logsBloom`. `reorg_safe_distance` is not needed
because only finalized blocks are served. Other pallets are added the same way, using
`pallet_address(key)` and `abis/<key>.abi.json`.

`abis/` is regenerated with:

```sh
SKIP_WASM_BUILD=1 cargo run -p substrate-eth-rpc -- emit-abi --out modules/utils/substrate-eth-rpc/abis
```

`tests/abi_golden.rs` fails when a committed ABI differs from the mapping table or when
`abis/` contains a file no pallet produces.

## Fixtures and tests

```sh
cargo run -p substrate-eth-rpc -- record-fixtures --config config.toml --chain hyperbridge-nexus --blocks 100,101,102 --out fixtures/nexus
```

records the named blocks from the live chain into a fixture directory that `FixtureRpc`
replays in tests without a network:

```
<out>/
  blocks/<number>.json     hash, parentHash, number, stateRoot, extrinsicsRoot, extrinsics,
                           storage, childStorage, runtimeVersion {specName, specVersion, transactionVersion},
                           parentRuntimeVersion (same shape, the runtime that produced the block's events)
  metadata/<specVersion>.scale
  finalized.json           { "number": <finalized head> }
```

Metadata is recorded for the block's own spec version and for the parent's, so an upgrade
block decodes in replay. The Nexus fixtures include the three runtime upgrades between
specs 8400 and 8700 (blocks 11755982, 12019344 and 12252354 with their parents).

`storage` holds the keys the server reads (`System.Events`, `Timestamp.Now`) and any
others a test needs, keyed by hex storage key. Fixture files contain no RPC URLs;
URLs stay in the environment, never in committed files.

Committed fixtures cover `hyperbridge-nexus` (mainnet) and `gargantua` (the Hyperbridge
testnet, `KUSAMA-4009`). Of the 18 mapped events, eleven appear in recorded blocks:
`StateMachineUpdated`, `Request`, `Response`, `PostRequestHandled`, `GetRequestHandled`,
`PostRequestTimeoutHandled`, `RelayerRewarded`, `Transfer`, `CollatorRewarded`,
`BandwidthCredited` and `BidPlaced`. `GetRequestTimeoutHandled`, `AccumulateFees`,
`Withdraw`, `BandwidthConsumed`, `SubscriptionEvicted`, `ForceCredited` and `TierSet` had
not been emitted on either chain when the fixtures were recorded and are covered by unit
tests with hand-built events only.

## Chain id convention

- Polkadot and Kusama parachains: `1_000_000_000 + para_id` (Hyperbridge Nexus
  `1000003367`, Gargantua `1000004009`).
- Standalone Substrate chains: `1_100_000_000 + n`, assigned in order.
