# 2026-09-05 — Live phantom bids live in the runtime state record, not the bid store

Chosen: the last phantom commitment per chain is persisted in `RuntimeState.phantomBids` and
restored before the filler starts. Measured on chain: every restart left one phantom deposit
(0.01 BRIDGE per chain) unretracted, and the retraction sweep could not recover them because
phantom bids are never written to the bid store.

Alternatives rejected: writing phantom bids into the bid store would put a bid per interval per
chain (hundreds a day) through the retraction sweep and its TTL logic, which is built around real
orders; retracting live phantom bids on graceful stop does nothing for crashes, which are the
restarts that matter. Reading this account's live bids back from the pallet would also recover the
deposits already stranded, but needs a storage query the SDK does not expose yet; it is the natural
follow-up.
