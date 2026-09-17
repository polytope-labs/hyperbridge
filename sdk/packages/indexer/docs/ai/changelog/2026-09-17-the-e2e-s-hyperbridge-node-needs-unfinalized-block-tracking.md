# 2026-09-17 — The E2E's Hyperbridge node needs unfinalized-block tracking

With the schema fixed, the E2E got as far as a queryable indexer and then timed out with every solver
still undiscovered: the fake orderbook logged exactly one request, and that one was the workflow's own
`curl`. The indexer never polled it.

The Hyperbridge node was crash-looping. It starts at the chain's head — which is what makes the poll
run at all, since the poll is skipped on blocks trailing wall clock by more than 120 s — and the best
header moves under it there. Without unfinalized-block tracking that trips an assertion inside
`UnfinalizedBlocksService`:

```
AssertionError: Expect best header and checking header to be at the same height
    at UnfinalizedBlocksService.getLastCorrectFinalizedBlock
```

`restart: always` then restarted it, and it polled only in the gaps between restarts.

Reproduced locally against the live Gargantua testnet and the fake orderbook, which also confirmed the
rest of the path. With `--unfinalized-blocks`: no crashes, 20 polls (the first `200`, the rest `304`
against the ETag) and three `SolverDiscoveryRequest` rows queued from the watchlist.

Also added, since this was invisible from the outside: `pollSolverWatchlist` now reports why it did
nothing — no URL, no block timestamp, or how far the block trails wall clock — at most once a minute.
A silent node that is working and a silent node that is skipping every block looked identical, which
is what made this take a second run to find.

`docker-compose.local.yml`'s Hyperbridge node does not pass the flag either and starts at the head the
same way. Left alone here rather than touched while chasing this, but it is the same exposure.

Files: `docker/docker-compose.solver-ci.yml`, `src/services/solverWatchlist.service.ts`
