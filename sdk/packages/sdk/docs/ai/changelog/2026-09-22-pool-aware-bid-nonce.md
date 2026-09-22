# 2026-09-22 — Submissions sign with a pool-aware nonce

`IntentsCoprocessor` serialises every extrinsic on its substrate account through a concurrency-1
queue, and each submission signed with polkadot-js's auto-nonce, which reads on-chain state. A
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

## Where the nonce comes from

`nextNonce` asks `system_accountNextIndex`, the only pool-aware source here, and passes the result
to `signAndSend`. `signAndSend`'s own `nonce: -1` is not pool-aware: `api.derive.tx.signingInfo`
prefers the `AccountNonceApi` runtime call when the runtime carries one, which Hyperbridge does, and
that call reads state. It only falls back to the RPC on a runtime without it.

The RPC counts the ready queue, so an extrinsic of this instance's that has left it — retried,
queued as future, or not counted yet — is covered by `lastPooledNonce`, the highest nonce this
instance has put into the pool. The next submission takes `max(rpc, lastPooledNonce + 1)`, so a
burst never reuses a nonce and a fresh instance still picks up the chain's. A node with no
`accountNextIndex` to ask signs against on-chain state, as before.

Replacement is unchanged. A retry for a stalled extrinsic still pins the nonce that extrinsic went
out under and doubles the tip, so exactly one of the two can execute. A stalled extrinsic whose
nonce was neither supplied nor readable is still returned rather than retried.
