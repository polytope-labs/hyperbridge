# 2026-08-26 — Substrate chains run on the Polytope build of the SubQuery node (#1163)

The stock `subquerynetwork/subql-node-substrate` image could not survive an RPC interruption: after a websocket drop it gave the endpoint five reconnect attempts and then exited, and any request that failed while the socket was down (a block fetch or a mapping handler read) took the process down immediately; both providers also cached rejected request promises, so retries replayed the stale error. Over http it never retried a 429 from the rate-limited hosted RPC. The substrate image is now `polytopelabs/subql-node-substrate:v6.4.7-0`, built from the `polytope-labs/subql` fork with those behaviours changed (no response caching, requests wait for the reconnect, unbounded reconnect with capped backoff, http retries honouring Retry-After with a client-wide pause). The EVM image is unchanged.

Files: `scripts/generate-compose.ts`, `docker/docker-compose.local.yml`, `docker/docker-compose.nexus-ci.yml`.
