# 2026-09-16 — The protocol fee is already out of an order by the time we price it

Decided: the matcher prices against `order.inputs[0].amount` as it arrives, with no fee adjustment
anywhere in the limit order path, and `bookPrice` is stored next to `price` rather than reconciled
with it.

`IntentGatewayV2.placeOrder` takes `protocolFeeBps` off the **input**: it escrows `X(1-f)` under a
commitment computed over the reduced inputs, and emits `inputs: reducedInputs`. So the order our
scanner reconstructs is already net of the fee, which is what `inputNet` in `fx.ts` is named for.

The orderbook shades the other side. `crates/core/src/haircut.rs` applies the fee to what the solver
delivers and quotes every price on that, so `bookPrice = amountOut(1-f) / amountIn`, where `price`,
the rate the op is signed at, is `amountOut / amountIn`.

Both are right, for different readers:

- the solver receives `X(1-f)` and pays `X(1-f) · price`, so it gets exactly the rate it signed and
  the fee never touches it;
- the swapper pays `X` and receives `X(1-f) · price`, so their rate is `price(1-f)`, which is
  `bookPrice`.

They are two sides of one trade rather than two estimates of one number, which is why both are kept
on the row and neither is derived from the other at read time.

What makes this worth recording is the failure it hides. If the gateway ever emitted gross inputs,
`inputNet` would quietly become gross, every payout would be `1/(1-f)` too large, and nothing here
would notice: the matcher takes the event at its word, and the golden vectors fix the op rather than
the event. The invariant to hold onto is that `OrderPlaced` carries reduced inputs.

Rejected: subtracting the fee ourselves before pricing. It would double-count today, and it would put
a second copy of the gateway's fee schedule in the filler, which is the thing the reduced event
exists to avoid.
