# 2026-11-15 — Indexer start-up: the Compose plugin, and finalized-only local nodes

## The start scripts use the Docker Compose plugin

`start:local`, `start:solver-ci` and `start:nexus-ci` now run `docker compose`, the Compose plugin
of the Docker CLI. Before this they ran `docker-compose`, the separate v1 binary.

GitHub's `ubuntu-latest` runners moved to Ubuntu 26.04, which has no `docker-compose` binary, so
the scripts failed with `docker-compose: not found` and the SDK `live` job timed out waiting for
the indexer.

The workflows that start the indexer no longer install a Compose package. They use the plugin the
runner image ships, and `docker compose version` fails the job early if it is missing.

Running these scripts on a developer machine needs the Compose plugin too.

## The local nodes index finalized blocks only, except Base Sepolia

`docker/docker-compose.local.yml` now starts the Hyperbridge (Gargantua), BSC Chapel and Polygon
Amoy nodes with `--unfinalized-blocks=false`. That matches the testnet deployment: the compose
generator passes the flag to every substrate node, and `config-testnet.json` and
`config-local.json` set `unfinalizedBlocks: false` for BSC Chapel and Polygon Amoy.

Without the flag set to false, these nodes followed the best head. When one saw a fork it
rewound, and a rewind in a multi-chain project makes every chain in the database rewind too. A
node that has not stored a block yet cannot rewind. It exits with
`Cannot reindex with missing lastProcessedHeight` on every start, and the other chains wait for
it without end. In CI the SDK `live` job then failed with
`Timed out waiting for the indexer to catch up`.

Rows from these three chains now appear once their block is finalized. Chapel finalizes a block
or two behind its head, and Amoy within a few blocks. Codegen already starts the Gargantua node
at the finalized head.

The Polygon Amoy node also gets `--block-confirmations=3`, as the other two EVM nodes have. It
applies when the node does not find the endpoint's `finalized` tag at start-up. The default
would then hold the node 200 blocks behind.

Base Sepolia still indexes unfinalized blocks, as on testnet, because its finalized block trails
the head by about 20 minutes. It is the only local node that still checks for forks, so the
stall needs it to see one before another node has stored its first block.

A local database that stalled this way stays stalled. Delete the directory `DB_PATH` points to
and start again.
