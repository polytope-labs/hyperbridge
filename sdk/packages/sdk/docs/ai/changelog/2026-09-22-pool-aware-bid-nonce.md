# 2026-09-22 — Submissions sign with the pool-aware nonce

`IntentsCoprocessor` serialises every extrinsic on its substrate account through a concurrency-1
queue, and each submission used the API's auto-nonce, which is the account's on-chain nonce. A
watch gives up while its extrinsic is still in the pool (`stalled`, reported as `pending`), and a
pooled extrinsic does not advance the on-chain nonce. The next, unrelated extrinsic then signed
that same nonce and bounced off the first:

```
1014: Priority is too low: (2908000000727 vs 734000000734): The transaction has too low priority
to replace another transaction already in the pool.
```

A bounce is reported as `pending`, so the caller never submitted. On testnet this lost a whole
auction: one solver's four bids for a multi-leg order each bounced off the retraction still pooled
ahead of them, and the order got no bid at all.

A new submission now signs with `nonce: -1`, the pool-aware next index, over the websocket and the
HTTP fallback alike. It queues behind anything pooled instead of colliding with it.

Replacement is unchanged. A retry for a stalled extrinsic still pins the nonce that extrinsic went
out under, and doubles the tip, so exactly one of the two can execute. Only a fresh submission
takes `-1`.
