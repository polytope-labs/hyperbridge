# Reads race every endpoint, and boot runs its per-chain steps side by side

A solver with five chains on the default public endpoints took over three minutes to start. Two
changes address that.

## Reads race

`ChainClientManager.raceTransport` is the public client's transport for a chain with more than one
endpoint. Every read is sent to all of the chain's endpoints at once, and the first answer is used.

- A result is an answer. So is a revert, by the same `shouldThrow` rule viem's `fallback` uses to
  stop failing over.
- An endpoint that is unreachable, times out, rate-limits or refuses the method is skipped. The read
  fails only when every endpoint has failed, and the transport then retries the whole race with
  viem's defaults.
- No endpoint retries on its own. The other endpoints are the retry.

Before this, the public client used `fallback` over endpoints that each retried three times with
1s, 2s and 4s of backoff. Every read started from the first endpoint, so each endpoint that was down
ahead of a live one added seven seconds to every read on that chain.

Consequences to keep in mind:

- Every listed endpoint now receives every read outside the quorum, where only the first one did.
- The first-listed endpoint is no longer the one that answers. Two consecutive reads can come from
  nodes at different heights.
- The wallet client is unchanged. It keeps the ordered `fallback`, so a transaction goes to one
  endpoint and the nonce read behind it reaches the same node.

A chain with a single endpoint keeps plain `http` with its retries.

## Per-chain boot steps run side by side

`IntentFiller.solverSelectionChains`, `DelegationService.setupDelegationOnChains` and the EntryPoint
deposit top-ups were loops that awaited one chain at a time. Each now runs its chains concurrently,
so boot waits for the slowest chain instead of the sum of all of them. Delegation state is keyed by
wallet and chain, so the chains do not share any.
