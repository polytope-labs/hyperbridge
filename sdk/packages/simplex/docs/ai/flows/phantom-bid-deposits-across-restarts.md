# Phantom bid deposits across restarts

`IntentFiller.handlePhantomOrders` submits one `forceBatch` per interval carrying each configured
chain's `placeBid` and, when `lastPhantomCommitmentByChain` has a previous bid for the chain, its
`retractBid` (refunding the 0.01 BRIDGE storage deposit). `rememberPhantomBid` updates that map on a
landed or pooled bid and persists it as `RuntimeState.phantomBids` through `patchRuntimeState`
(`Simplex.pause/resume` and the CLI's `setPaused` use the same helper, and the SQLite store's
`patch` writes only the key it is given, so neither drops the other's — see
[operator state on disk](./operator-state-on-disk.md)). `bootFiller` reads the state before
starting and calls
`restorePhantomBids`, so the first batch of a new process retracts the bid the previous process
left live. A chain already remembered by the running process is not overwritten by the restore.
