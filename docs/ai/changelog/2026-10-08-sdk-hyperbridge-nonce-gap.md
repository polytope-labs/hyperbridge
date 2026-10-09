# SDK: Hyperbridge nonce gap after a dropped extrinsic

`IntentsCoprocessor` signs each bid and retract at `max(system_accountNextIndex, lastPooledNonce + 1)`,
so a burst never reuses a nonce it has already put into the pool. If the node later drops one of
those extrinsics without including it, every later submission sits in the pool's Future queue
behind the missing nonce. `sendWithTimeout` marks a watch that timed out with `Future` as the
last pool status (`future`, next to `stalled`).

On such a stall, `sendExtrinsicWithRetries` reads `system_accountNextIndex` from the same node. If it
is below the stalled nonce, a zero-tip `system.remark` signed at that index fills the gap, and the
retries carry on at the pinned nonce. A remark is used rather than the next call so the extrinsics
already pooled above the gap land in the order they were signed. With no tip, a live call of ours
still holding that nonce on another node or pool view always outranks the remark. Each stall fills
one nonce, and the fill gives up after the attempt's timeout. Nothing is sent for a stall in Ready,
or when the index has reached the stalled nonce. A remark that fails or times out is logged, and
the call's retries and result are the same as without it.
