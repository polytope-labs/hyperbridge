# 2026-08-28 — The legacy `fillOrder` ABI is exported, not re-declared downstream

The indexer needs the v1 shape to decode bids with ethers. Two ways to give it one:

- **Re-declare it there.** No SDK change, but then two definitions of the same historical ABI have to stay
  identical forever, with nothing enforcing it. The bug being fixed is precisely a decoder that fell out of step
  with the shapes in the wild, so adding a second source of truth for those shapes is the wrong direction.
- **Export the existing constant.** One definition. Costs a public export of something that is otherwise an
  implementation detail — acceptable, since the reason it exists (deployments predating `validUntil`) is a fact
  about the network rather than about this package.

Exported from `fillOrderCodec` and re-exported on the `intents-helpers` sub-path, which is the entry point that
exists for tools that cannot load the full bundle. Pulling `fillOrderCodec` onto that sub-path adds no new
runtime dependency — it imports only viem, the gateway ABI, and types, all already reachable there.

`decodeFillOrder` is deliberately not exported alongside it: it is viem-based, and the callers that need this
constant are the ones that cannot run viem.
