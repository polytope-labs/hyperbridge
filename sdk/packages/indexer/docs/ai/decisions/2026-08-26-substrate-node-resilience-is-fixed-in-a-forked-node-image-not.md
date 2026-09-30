# 2026-08-26 — Substrate node resilience is fixed in a forked node image, not in the indexer

Chosen: the substrate SubQuery node image comes from the `polytope-labs/subql` fork (`polytopelabs/subql-node-substrate`), where the websocket provider is wrapped so requests wait out a disconnect, response caching is disabled, reconnects are unbounded, and the http provider retries rate limits with a client-wide pause. The indexer package only changes the image reference.

Alternative rejected — make the indexer tolerate it: retry handler RPC reads, lower `--workers`, rely on `restart: unless-stopped`. The exits come from inside the node (block fetcher and dispatcher), which handler code cannot reach, and a restart is not a reconnect: it drops the unfinalized cache and, under `--multi-chain`, forces a rewind. The rate limit on the hosted http RPC is per client, so per-call retries in handlers cannot coordinate with the node's own fetch traffic; only the provider can pause all of them.

Alternative rejected — upstream the changes first and wait. Worth doing, but the deployment needs the behaviour now; the fork mirrors how `polytopelabs/subql-node-ethereum` is already produced from `polytope-labs/subql-ethereum`.

Accepted: this moves substrate from the deployed node 5.9.1 to 6.4.7 (node-core 19.x), matching `polytope-labs/subql`'s main after it was synced to upstream. The fork PR is built on that main, so it carries only these behavioural changes as a single commit, not the version history.
