# 2026-11-15 — The indexer start scripts use the Docker Compose plugin

`start:local`, `start:solver-ci` and `start:nexus-ci` now run `docker compose`, the Compose plugin
of the Docker CLI. Before this they ran `docker-compose`, the separate v1 binary.

GitHub's `ubuntu-latest` runners moved to Ubuntu 26.04, which has no `docker-compose` binary, so
the scripts failed with `docker-compose: not found` and the SDK `live` job timed out waiting for
the indexer.

The workflows that start the indexer no longer install a Compose package. They use the plugin the
runner image ships, and `docker compose version` fails the job early if it is missing.

Running these scripts on a developer machine needs the Compose plugin too.
