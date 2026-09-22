# 2026-09-16 — A fill resizes its limit order

A fill now works its limit order down by what actually went out and puts the rest back on the
orderbook, so the depth the book advertises is output the order still has.

## Getting the amounts

`OrderFilled` and `PartialFill` both carry `outputs` and `inputs`, and neither reached the filler.
`ScannedFill` carries them now, `ChainScanner` reads them off the log, and `EventMonitor` passes them
through `orderFilledOnChain`.

A fill that carries none of the token its limit order pays leaves the order untouched, with a warning.
Sizing a draw-down from a guess would either advertise output already paid or quietly retire output
still available, and both are worse than an order that looks unchanged until reconciliation notices.

## Settling

On a fill of ours, `IntentFiller` claims the bid's reservation, gives the hold back, and works the
order down by the delivered amount normalized to 1e18. Those are two different numbers: the hold is
what the bid promised, the delivery is what the gateway recorded going out, and taking one off
`reserved` while taking the other off `remaining` is what leaves the order describing what it still
has.

`LimitOrderService.settleFill` then either closes the order, when what is left is under the
orderbook's dust floor for the token it pays, or reposts it at its new size. The status moves to
`resizing` and the new size is written before the orderbook is touched, so a crash mid-way leaves a
row that says what it was doing.

The old entry is cancelled before the new one is posted, because two live entries for one liability
would advertise the same output twice; the gap between the two calls is at most one request. An
`UNKNOWN_ORDER` on the cancel is as good as cancelled. Every repost signs on a fresh nonce, since the
orderbook remembers every op hash it has accepted.

`repost` is separate from `settleFill` because reconciliation after a
restart both want the same thing: whatever the order has left, live on the book again.
