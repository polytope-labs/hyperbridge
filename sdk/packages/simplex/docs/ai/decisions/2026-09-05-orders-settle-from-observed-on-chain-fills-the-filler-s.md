# 2026-09-05 — Orders settle from observed on-chain fills; the filler's `orderFilled` stays as is

Chosen: keep emitting `orderFilled` on an accepted bid (the library's `order:filled` event and its
consumers are unchanged) but tag it with the commitment, and derive the order's real outcome from
the OrderFilled log via a new `orderFillObserved` event. The maintainer saw a rival-filled order
shown as Filled.

Alternatives rejected: renaming or suppressing `orderFilled` for bids changes a public event that
integrators may count on; treating the bid's extrinsic hash as a fill tx (the old behaviour) sent
the fill link to a hash no EVM explorer knows. Rival fills are recorded only for orders this filler
already has rows for, because every fill on the chain passes through the monitor and a feed of
other lanes' fills would bury this filler's own history.
