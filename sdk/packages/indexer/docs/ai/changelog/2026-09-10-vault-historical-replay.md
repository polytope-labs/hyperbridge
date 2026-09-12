# 2026-09-10 — Historical Docker replay reproduces and fixes inflated vault yield

Ran the actual SubQuery Ethereum node against Base archive data from the premium Alchemy RPC,
with PostgreSQL persistence and the original vault handlers. The pre-fix control reproduced the
reported API result exactly. The fixed build matched an independent calculation from the original
event logs and historical contract calls, down to one raw USDC unit.

## Inputs and scope

- Wallet: `0xce319986ca4d5d0893751a628d0db3dc8fc91d62`.
- Vault: `0xc768c589647798a6ee01a91fde98ef2ed046dbd6` on Base (`EVM-8453`), wrapping USDC.
- Control source: `5d0e05806e6690c716de02e58e8401b609b93cd1` (`main` before the fix).
- Fixed source: `ea8ba850af19e2b6906ab1a0ee4fb0716843cc4a`.
- Node: `subquerynetwork/subql-node-ethereum:v6.5.0`, image digest
  `sha256:f1ba196ce1da75d2a34ac78c940e77c457c6c8027997c11055fd6ed7794732e7`.
- Database: `postgres:14-alpine`, separate fresh `control` and `fixed` schemas, historical indexing
  by height, synchronous store flushing. No provider, ledger, position, or snapshot rows were seeded.

An archive log scan covered blocks 51,090,774 through 51,104,550 inclusive. It found 27 logs on this
vault in 13 blocks. The replay manifests retained those event blocks and explicitly bypassed the
intervening blocks with no vault logs. Both runs invoked the original snapshot handler at block
51,104,550 (2026-09-10 00:00:47 UTC), the same block as the incorrect production snapshot. The node
also fetched intermediate boundary blocks while advancing its checkpoint; both runs finished at
51,104,550 and exited successfully.

This is a scoped vault integration replay: other protocol event datasources and cross-chain provider
registration indexing were not run. Handler/service code was unchanged in isolated build copies;
the generated RPC configuration pointed to a local proxy forwarding read-only requests to Alchemy.
No contract responses or database operations were mocked. All contract, balance and delegation
reads used explicit historical block tags. No chain transactions or production database writes occurred.

## Original wallet events

| Block | Event | Shares received | USDC deposited |
| --- | --- | ---: | ---: |
| 51,090,775 | Ordinary share Transfer | 174220559508 | 0 |
| 51,090,824 | Ordinary share Transfer | 57243722070 | 0 |
| 51,091,762 | Deposit (plus its mint Transfer) | 17666436 | 20.230586 |

These were the only wallet share movements in the scanned window. Its opening share balance before
the first transfer was zero. `eth_getCode` returned `0x` at both transfer blocks and delegation to
`0x7cb55539d1144f62422099c3fa3405092022c88c` at the deposit block.

With no existing provider row, both fresh runs therefore first tracked this wallet at the deposit.
The fixed run established opening shares of `231464281578`, valued at `265059572508` raw USDC
(265,059.572508 USDC) at block 51,091,762. It then recorded the real deposit exactly once. It did not
invent historical deposits or transfer ledger rows for the earlier, ineligible period.

## Persisted results at block 51,104,550

Amounts below are USDC; share counts are raw integers.

| Field | Pre-fix control | Fixed |
| --- | ---: | ---: |
| Position shares | 17666436 | 231481948014 |
| Snapshot shares (onchain) | 231481948014 | 231481948014 |
| Snapshot asset value | 265,087.704647 | 265,087.704647 |
| Opening principal | Not recorded | 265,059.572508 |
| Actual deposits | 20.230586 | 20.230586 |
| Net principal | 20.230586 | 265,079.803094 |
| Reported yield | **265,067.474061** | **7.901553** |
| Deposit / withdrawal count | 1 / 0 | 1 / 0 |

The control yield equals the incorrect live API snapshot captured during the original investigation.
Independent assertions checked both database checkpoints, ledger counts, event types, deposited
assets, opening shares/principal/block, tracked shares, counters, snapshot values and yields. All passed.

The earlier receipt-time reconstruction gives **8.497566 USDC**. That calculation starts at the two
original transfers. This fresh indexer run starts its opening basis at the later first eligible event,
excluding 0.596013 USDC of pre-baseline appreciation. This is the documented opening-principal
behavior, not an exact lifetime-yield backfill. Historical rows in an existing deployment still require
the separate repair; this run does not test that repair or the upgrade of an existing database.

## Local evidence

The test workspace is `/tmp/hyperfx-historical-replay`. Its `compose.yaml`, isolated source builds and
bounded `src/configs/replay.yaml` manifests describe the run. The `evidence` directory contains:

- `vault-logs.json`, `wallet-logs.json`, and `onchain-states.json`: original archive inputs.
- `independent-expectations.json`: separately calculated expected balances, principal and yields.
- `fixed-db.json` and `control-db.json`: persisted wallet positions, ledgers and snapshots.
- `rpc-requests.jsonl`: methods and parameters, without the premium credential.
- Build and container logs for both runs.

`python3 /tmp/hyperfx-historical-replay/verify-results.py` checks the saved database exports against
the independent expectations and verifies historical RPC tags. The local Docker database volume is
retained for inspection. Replay containers were removed and the temporary RPC credential file cleared
after collecting evidence. The user's existing Docker services were left untouched.
