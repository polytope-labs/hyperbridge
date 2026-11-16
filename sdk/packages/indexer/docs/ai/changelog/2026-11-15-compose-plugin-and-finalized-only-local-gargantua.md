# 2026-11-15 — Indexer start-up: the Compose plugin, and a finalized-only local Gargantua node

## The start scripts use the Docker Compose plugin

`start:local`, `start:solver-ci` and `start:nexus-ci` now run `docker compose`, the Compose plugin
of the Docker CLI. Before this they ran `docker-compose`, the separate v1 binary.

GitHub's `ubuntu-latest` runners moved to Ubuntu 26.04, which has no `docker-compose` binary, so
the scripts failed with `docker-compose: not found` and the SDK `live` job timed out waiting for
the indexer.

The workflows that start the indexer no longer install a Compose package. They use the plugin the
runner image ships, and `docker compose version` fails the job early if it is missing.

Running these scripts on a developer machine needs the Compose plugin too.

## The local Gargantua node indexes finalized blocks only

`docker/docker-compose.local.yml` now starts the Hyperbridge (Gargantua) node with
`--unfinalized-blocks=false`. The generator for the testnet and mainnet compose files already
passes it to every substrate node.

This node indexes unfinalized blocks unless that flag is false, so leaving it out made the local
node follow the best head. When it saw a fork it rewound, and a rewind in a multi-chain project
makes every chain in the database rewind too. An EVM node that has not stored a block yet cannot
rewind. It exits with `Cannot reindex with missing lastProcessedHeight` on every start, and the
other chains wait for it without end. In CI the SDK `live` job then failed with
`Timed out waiting for the indexer to catch up`.

Rows from Gargantua now appear once their block is finalized. Codegen already starts the node at
the finalized head.

The three local EVM nodes still index unfinalized blocks. A fork on one of them causes the same
stall if another has not stored its first block yet. They start together, so that window is
short.

A local database that stalled this way stays stalled. Delete the directory `DB_PATH` points to
and start again.
